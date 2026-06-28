## multiplayer_fps

### Instructions

Write your own version of the game [maze wars](https://www.youtube.com/watch?v=5V5X5SbSjns). You should recreate all the elements of the game, but you have freedom to implement the user interface.

### Objectives

#### User Interface

The game should present a specific User Interface, in which the minimum requirements are:

- A mini map where the player can see his own position and the whole "game world".
- The graphics of the game (walls and other players) should be similar to the original game (see [maze_wars](https://www.youtube.com/watch?v=5V5X5SbSjns) for more details)
- Finally you have to display the frame rate of the game on the screen.

The client must be implemented with the [Bevy](https://bevyengine.org/) game
framework. Use Bevy for the application loop, rendering, input, UI, audio, and
entity/component management. The server may use Bevy's headless ECS/scheduling
facilities, but it must not require a window or graphics device.

Render the game with sprites aligned to a common 32 x 32 pixel tile grid. Use
these standard source-asset sizes so that scaling and collision bounds remain
consistent:

- Floor and wall tiles: 32 x 32 pixels.
- Players: 32 x 32 pixels, centered in one tile.
- Projectiles: 8 x 8 pixels, centered inside their collision bounds.
- Pickups and other tile occupants: 16 x 16 or 32 x 32 pixels.

Prefer a sprite atlas whose regions follow these dimensions. Apply nearest
neighbor sampling and integer scaling to preserve crisp edges. World-space
movement may be continuous even though walls and level generation use the tile
grid.

#### Controls

The default keyboard controls are:

- `W`, `A`, `S`, and `D`: move forward, left, backward, and right.
- Left and right arrow keys: rotate counterclockwise and clockwise.
- Up and down arrow keys: rotate the view up and down when the chosen camera
  representation supports pitch; otherwise they must remain bound as rotation
  inputs and may be used for discrete look/turn behavior.
- Space: shoot.

Movement and rotation must be frame-rate independent. The client sends input
intent to the server; the server remains authoritative for movement,
collisions, projectile spawning, hits, and player state. Controls should be
configurable in one input-mapping resource rather than being scattered across
gameplay systems.

#### Architecture

- Implement a client-server architecture where clients connect to a central server to play the game.
- Your implementation should allow one client and the server to run on the same machine, with other clients connecting from different machines.
- Use the UDP protocol to enable the communication between the clients and the server.
- The game should have at least 3 levels with increasing difficulty (with difficulty we mean, making the maze harder, with more dead ends).

#### Procedural world generation

All playable maps must be generated procedurally. The authoritative server
selects and records a seed, generates the map once, and sends either the seed
plus generator version or the resulting map data to every client. Clients must
not independently choose map topology.

Generation must satisfy all of the following invariants before a level starts:

- Every player spawn is on a walkable tile and has at least two walkable
  neighboring tiles, so a player cannot spawn inside a wall or sealed pocket.
- A flood-fill from every spawn reaches every other spawn and every required
  gameplay objective. There must be exactly one connected playable region for
  tiles used by players.
- Spawn collision bounds have enough clearance for the 32 x 32 player sprite.
- Spawns are separated by a configurable minimum path distance and do not have
  an immediate unobstructed firing line to another spawn.
- The generator has a bounded retry count. If a candidate fails validation, it
  retries with a derived seed; if all retries fail, it loads a known-valid
  generated fallback map instead of starting an invalid level.

A spanning-tree maze algorithm such as randomized depth-first search, Prim's,
or Kruskal's algorithm can guarantee base connectivity. Extra loops should then
be carved to reduce trapping and improve multiplayer movement. Difficulty may
increase through larger maps, longer routes, and progressively more dead ends,
but it must never weaken the connectivity and spawn-safety checks. Store the
seed in logs so any generated level can be reproduced during testing.

You will have to develop the game server and also a client application:

- The server must accept as many connections as possible (the minimum should be 10).
- When the client is initialized, the game should ask for:
  - The IP address of the server, allowing the client application to connect to any server.
  - A username for identification.

After providing the above information, the game should start and open the graphical interface, allowing the player to join and start playing the game.

Example:
Assuming that you can connect to a server in the same computer.

```console
$ cargo run
Enter IP Address: 198.1.1.34:34254
Enter Name: name
Starting...
$
```

#### Performance

The game should always have a frame rate above 50 fps (frames per second).

### Bonus

As bonus for this project here are some ideas:

- Implement a level editor to allow players to create their own mazes.
- Implement an algorithm that generates automatically new mazes.
- Implement A.I. players to allow playing the game without having to wait for more people to join to the server.
- For the basic implementation you can initialize the game from the command line. As a bonus you can implement the initialization of the game as part of the graphical interface and save a history of the hosts with an alias so it's easier to reconnect to known servers.

This project will help you learn about:

- GUI applications
- Game mechanics
- [UDP protocol](https://searchnetworking.techtarget.com/definition/UDP-User-Datagram-Protocol)
