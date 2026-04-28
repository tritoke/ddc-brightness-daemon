# ddc-brightness-daemon

A daemon+frontend to control the brightness of external monitors via DDC/CI commands.

## Basic usage

Once the daemon is running you can use `brightctl` to change the brightness of your screens:
```sh
# get the brightness of all screens (also the default behaviour)
brightctl --get
# list metadata about the screens
brightctl --list

# set all screens to full brightness
brightctl --set 100
# make all screens 10% brighter
brightctl --inc 10
# make all screens 10% dimmer
brightctl --dec 10

# all of the above also accept -d/--display for setting a specific monitor, e.g.
brightctl --set 100 --display 1
```

## Installing and running the daemon
Everything can be installed from cargo: `cargo install --locked ddc-brightness-daemon`,
this gets you both `brightctl` and the daemon `ddc-brightness-daemon`.

The daemon is instead to be run as your user with a systemd service file, however
you may run it however you please.

To create a service file for the daemon you can use the below template:
```systemd
[Unit]
Description=DDC Brightness Daemon
StartLimitIntervalSec=0
After=multi-user.target
Wants=multi-user.target

[Service]
Type=simple
ExecStart=%h/.cargo/bin/ddc-brightness-daemon

[Install]
WantedBy=default.target
```

```sh
# Create the service with edit, just paste the snippet above
systemctl --user edit --full --force ddc-brightness-daemon.service
# Reload systemd
systemctl --user daemon-reload
# Enable and start the service (this will start it automatically on boot)
systemctl --user enable --now ddc-brightness-daemon
```

## Documentation
The `brightctl` CLI is documentated in the `brightctl.1` man page, the D-Bus interface provided by
`ddc-brightness-daemon` can be introspected by `busctl` but currently has no proper documentation:
```sh
❯ busctl --user introspect org.tritoke.Brightness1 /org/tritoke/Displays org.tritoke.Displays
NAME                TYPE   SIGNATURE RESULT/VALUE FLAGS
.ChangeRelative     method tn        -            -    
.GetDisplayMetadata method -         aa{ss}       -    
.ListBrightness     method t         q            -
.SetAbsolute        method tq        -            -
```

`busctl` can also call into the interface:
```sh
❯ busctl --user call org.tritoke.Brightness1 /org/tritoke/Displays org.tritoke.Displays GetDisplayMetadata
aa{ss} 3 6 "model_name" "VG27AQL1A" "manufacture_year" "2020" "manufacture_week" "46" "model_id" "2705" "manufacturer_id" "AUS" "serial" "01010101" 6 "manufacture_week" "5" "model_id" "5BD3" "manufacture_year" "2021" "model_name" "LG ULTRAGEAR" "manufacturer_id" "GSM" "serial" "0001E031" 6 "manufacturer_id" "GSM" "model_id" "5B7F" "serial" "00066678" "model_name" "27GL850" "manufacture_week" "11" "manufacture_year" "2020"
```

Using the client is recommended however as the output is not particularly human friendly.
