use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::BufRead;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

use std::collections::HashMap;

use rpassword::read_password;

use reqwest::blocking::Client;
use reqwest::header::COOKIE;
use reqwest::Url;

#[derive(Subcommand, Debug, Clone)]
enum Commands {
	SetConf {
		///url of phpipam instance, e.g http://127.0.0.1:1234
		#[arg(long)]
		url: String,

		///ipam username
		#[arg(short, long)]
		username: Option<String>,

		///ipam password
		#[arg(short, long)]
		password: Option<String>,
	},
	Search {
		///query text
		#[arg(short, long)]
		query: String,

		///if true, only hosts are printed
		#[arg(short, long)]
		hosts_only: bool,

		///if true, only ip addresses are printed
		#[arg(short, long)]
		ips_only: bool,
	},
}

#[derive(Parser, Clone)]
#[clap(author, version, about, long_about = None)]
struct Args {
	#[clap(subcommand)]
	command: Commands,
}

#[derive(Default, Debug)]
struct Conf {
	url: String,
	username: Option<String>,
	password: Option<String>,
}

fn parse_conf(conf: &String) -> Result<Conf, String> {
	let mut result: Conf = Conf::default();
	for line in conf.lines() {
		let mut iter = line.split("=");
		let fname = iter.next().ok_or("unexpected")?;
		let value = iter.collect::<Vec<&str>>().join("=");
		match fname {
			"url" => {
				value.parse::<Url>().expect("Invalid url");
				result.url = value;
			}
			"username" => result.username = Some(value),
			"password" => result.password = Some(value),
			_ => eprintln!("Unexpected param: {}", fname),
		};
	}
	Ok(result)
}

fn get_token(config: &Conf) -> Result<String, Box<dyn std::error::Error>> {
	let username = config.username.clone().unwrap_or_else(|| {
		print!("username: ");
		std::io::stdout().flush().unwrap();
		let stdin = io::stdin();
		let mut it = stdin.lock().lines();
		it.next().unwrap().unwrap()
	});
	let password = config.password.clone().unwrap_or_else(|| {
		print!("password: ");
		std::io::stdout().flush().unwrap();
		read_password().unwrap()
	});

	let client = Client::new();
	let mut params = HashMap::new();
	params.insert("ipamusername", username);
	params.insert("ipampassword", password);
	params.insert("phpipamredirect", "/".to_string());
	let response = client
		.post(format!("{}/app/login/login_check.php", config.url))
		.form(&params)
		.send()?;

	assert!(response.status().is_success());

	let token = response.cookies().last().expect("fail").value().to_string();
	//let body = response.text()?;

	Ok(token)
}

fn search(
	query: String,
	token: String,
	config: &Conf,
) -> Result<String, Box<dyn std::error::Error>> {
	let client = Client::new();
	let mut params = HashMap::new();
	params.insert("page", "tools");
	params.insert("section", "search");
	params.insert("ip", &query);
	let response = client
		.get(format!("{}/index.php", config.url))
		.query(&params)
		.header(COOKIE, format!("phpipam={}; table-page-size=50; search_parameters=%7B%22addresses%22%3A%22on%22%2C%22subnets%22%3A%22on%22%2C%22vlans%22%3A%22on%22%2C%22vrf%22%3A%22off%22%2C%22pstn%22%3A%22off%22%2C%22circuits%22%3A%22on%22%2C%22customers%22%3A%22off%22%7D", token))
		.send()?;

	Ok(response.text()?)
}

fn parse_search(result: &String) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
	let mut hosts = HashMap::new();
	let mut it = result.lines().map(|line| line.trim());
	loop {
		match it.next() {
			Some(line) => {
				if !line.contains("<td class=\"ip\">") {
					continue;
				}
				let ip = line
					.split("</a>")
					.next()
					.expect("Couldnt split")
					.split(">")
					.last()
					.expect("No ip")
					.to_string();
				it.next();
				let hostname: String = it
					.next()
					.expect("No hostname after ip")
					.replace("<td>", "")
					.replace("</td>", "")
					.to_string();
				hosts.insert(hostname, ip);
			}
			None => break,
		}
	}
	Ok(hosts)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	let home_dir = dirs::home_dir().expect("Unable to find home directory");
	let mut cnfpath = PathBuf::from(home_dir);
	cnfpath.push("phrustpam.cnf");

	match args.command {
		Commands::SetConf {
			url,
			username,
			password,
		} => {
			let mut file = File::create(&cnfpath)?;
			url.parse::<Url>().expect("Invalid url");
			let data = format!(
				"url={}\n{}{}",
				url,
				username.map(|s| format!("username={}\n", s)).unwrap_or(String::new()),
				password.map(|s| format!("password={}\n", s)).unwrap_or(String::new()),
			);
			file.write_all(data.as_bytes())?;
		}
		Commands::Search {
			query,
			hosts_only,
			ips_only,
		} => match cnfpath.exists() {
			false => {
				eprintln!("Not configured, use setconf subcommand first.");
				process::exit(1);
			}
			true => {
				if hosts_only && ips_only {
					eprintln!("Cannot have both hosts only and ips only");
					process::exit(1);
				}

				let mut file = File::open(&cnfpath)?;
				let mut contents = String::new();
				file.read_to_string(&mut contents)?;
				let conf = parse_conf(&contents)?;

				eprintln!("Logging in");
				let token = get_token(&conf)?;
				eprintln!("Sending query");
				let result = search(query, token, &conf)?;
				let hosts = parse_search(&result)?;
				for (hostname, ip) in hosts {
					let outputline = match (hosts_only, ips_only) {
						(false, false) => format!("{} {}", hostname, ip),
						(false, true) => format!("{}", ip),
						(true, false) => format!("{}", hostname),
						_ => unreachable!(),
					};
					println!("{}", outputline);
				}
			}
		},
	}
	Ok(())
}
