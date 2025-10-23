# Custom targets
Custom targets are targets that has a suffix that starts with a `#`. These are user defined targets that can be enabled programmatically.
Custom targets are the highest priority targets in Tuckr, so they will allows override any other group.

They were created to be used as a way to deploy different variants of a file under the same platform. But you can use it for whatever application you may find for them.

For example if you're running on Linux but have a raspberry pi and a desktop and want different configs to be used for a certain program you can create a `group_#raspberrypi` and `group_#dekstop`.
Then you can pick which to choose by using `tuckr -t raspberrypi add group` to tell tuckr that raspberry pi is a valid target right now. 

If your custom target is a more permanent target you can use the `TUCKR_CUSTOM TARGETS` environment variable or create a tuckr alias in your shell that adds the flag.

Both the flag and the env variable support defining multiple targets at once by separating them with a comma:
```
$ tuckr -t raspberrypi,desktop add group
$ TUCKR_CUSTOM_TARGETS="raspberrypi,desktop" tuckr add group
```

