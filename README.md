# phrustpam
The CLI for [phpipam](https://phpipam.net/) you didn't ask for.

## Usage

### Sub commands
```
Usage: phrustpam <COMMAND>

Commands:
  set-conf
  search
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Set config
```
Usage: phrustpam set-conf --url <URL> --username <USERNAME> --password <PASSWORD>

Options:
      --url <URL>            url of phpipam instance, e.g http://127.0.0.1:1234
  -u, --username <USERNAME>  ipam username
  -p, --password <PASSWORD>  ipam password
  -h, --help                 Print help
```
This writes config to `~/phrustpam.cnf`

### Query
```
Usage: phrustpam search [OPTIONS] --query <QUERY>

Options:
  -q, --query <QUERY>  query text
  -h, --hosts-only     if true, only hosts are printed
  -i, --ips-only       if true, only ip addresses are printed
  -h, --help           Print help
```

## How it works
This tool does not use the API, instead it attempts to parse the HTML intended for GUI.

## Improvements
Needs more testing accross different phpipam versions.
