# pepe-rust-bot

Dank Memer Bot made in Rust

## Usage

This bot is very simple to use.

1. [Download](https://github.com/hubble459/pepe-rust-bot/releases/latest) the pre-compiled binary for your platform;
2. Run `pepe-bot[.exe]` in a terminal;
3. Use `pepe-bot --help` for more info.

Example

```console
pepe@dank:~$ pepe-bot -h
...omitted
pepe@dank:~$ pepe-bot -V
pepe-bot 1.1
pepe@dank:~$ pepe-bot -m 000000000000000000 -t nHSdck.qwef-2c.wknefjqj -vvv
[running output]
```

Press `CTRL-C` to stop the program.

In Discord the master can use `@[bot_name] start` in any desired channel. This is where the bot will start farming.

To stop it you can use `@[bot_name] stop`.

## Development

The easiest way is to have a .env file containing your discord user token and master account id.

```properties

TOKEN=<discord user token>
MASTER_ID=<user id of master>

```

Run the program with the following command

```apache

cargo run

```

## Build

Build for your own platform.

```apache

cargo build [--release]

```
