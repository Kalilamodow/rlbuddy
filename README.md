# rlbuddy

An overlay which displays the ranks of everyone in your lobby. (and more coming soon!!!)

<img src="demo.jpeg">

> usage in casual (outdated)

## setup

Once you've acquired a binary, you'll need to make sure that the stats api is enabled. In `<Install Dir>\TAGame\Config\TAStatsAPI.ini`, set `PacketSendRate` to 1. Technically, any number 0-120 works, but 1 is really all you need. (My line 3 looks like `PacketSendRate=1.0`).

## usage

The app will connect to rocket league whether it's already running or it starts up later. It automatically pops up when a match starts, but you can manually open it by holding `Alt`. It won't take away focus from the game unless you actually click on the overlay, though.
