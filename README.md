## Memer

This Discord Bot was a for fun project to try out rust and test some language features. So please don´t be surprised 
when some things seem a bit odd and are by no stretch of the Imagination idiomatic rust :).

The main feature of this bot is to turn any embedded image or video from either reddit or youtube into an uploaded 
Image/Video. Why ? Because it looks nicer. Does it waste resources by dumping files and Video on Discord's storage 
servers. Yes.

Should you whish to use this bot on your own Discord server its quite easy.

All you have to do is download the repo, and provide a properties.toml file in the resources directory.
You now can compile the bot using cargo. If you want to run the bot on a Linux server you might have to compile it on the server. 
If you try to cross compile it from a mac you have to take care of the openSSl library. You have to statically link it.

To run the binary on a server just start it as a systemd services.

