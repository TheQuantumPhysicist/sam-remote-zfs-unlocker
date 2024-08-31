# Sam-remote-ZFS-Unlocker

## What is this?

This is a tool that can do two things with a nice web frontend:

1. Run arbitrary pre-determined commands in a remote terminal
2. Remotely unlock ZFS datasets

## What problem does it solve?

If you want to define commands that can be run on a web interface with one click or unlock and mount encrypted ZFS datasets, then this program does that.

A simple example is: Imagine your system crashes for any reason, and now your ZFS encrypted datasets are not unlocked. This software gives you the tools to be able to unlock the datasets with minimal efforts.

## But why not just use a terminal?

Because this program isn't supposed to be only for the tech-savvy. You can define special diagnostics commands or decryption commands for non-techy people and ask them to use it to solve some problems if you're not available.

## Can I run this in docker?

You shouldn't! This software is made to be run raw on the system. The web frontend can be run in docker, but I haven't set that up. How are you gonna unlock and mount ZFS datasets in a docker container? I don't think that's possible.

## Security

### Root access

I do not recommend giving the user running this program (the API server that's in charge of running the commands) complete root access (with sudo, for example).

I recommend creating a special user with limited access, and setting up sudo exceptions using `visudo`, where only the commands in question can be run by that user.

### Networking security

This program is designed to run within your home network and/or behind a VPN, and this is why it doesn't support authentication. DO NOT make this publicly accessible.

## How to run this software?

### Components

This software has two components:

1. Frontend/WebApp: Is what will be shown to the non-techy user with simple buttons. It's a program written with Rust, Leptos that produces a client-side WASM app to run in the browser. It connects to the API server and learns from it what it can show the user. The frontend can be configure with `app-config.toml`, where the base URL of the backend should be found.
2. **The API server**: The server that receives commands from the Frontend/WebApp. It can be configured to run custom commands. See the examples `api-config.toml`.

### How it works

The API server is supposed to be running in the background constantly. It can receive API requests through some port (default is 6677). The frontend loads in the browser, loads its configuration, and uses that configuration to know where to find the API server. Then, the frontend connects to the API server, and asks it for what commands it can run, and what ZFS commands it can run.

### How to run:

There's still no packaged version of the software. Maybe I'll do this later if enough people ask for it. Right now this software solves my own problems.

The API server can be run with:

1. Copy `api-config.toml.example` to `api-config.toml`, and configure API server with your desired commands. There are examples in it.
2. Copy `app-config.toml.example` to `app-config.toml`, and configure the base URL setting. (Note: There are mock settings that I use for testing the frontend. You can ignore them. Just set the base URL and that's all you need)
3. To run the API server, run the command:

```
cargo run --bin webserver -- server --config-path webserver-lib/api-config.toml
```

4. Make sure you have trunk installed: `cargo install trunk`
5. Enter the directory `frontend`, and launch the frontend:

```
trunk serve --open
```

### Firewall config

Note: The frontend runs locally on whatever machine you open the frontend with in the browser. Hence, the base URL configuration must point to the API server machine, and the firewall ports there should be open.

### Contributing

I love solving my problems with Rust, as I enjoy coding. I didn't make this software with huge plans in mind. It already serves the purpose I created it for. You're welcome to reasonably contribute.
