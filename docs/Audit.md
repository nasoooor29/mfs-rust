#### Functional

##### Inspect the project configuration and source code.

###### Is Bevy used as the client game framework for rendering, input, UI, and the application loop?

###### Does the server run without requiring a window or graphics device?

##### Try to run the game server

###### Does it compile and run without any warnings?

##### Try to run a client in the same computer as the server.

###### Does it compile and run without any warnings?

###### Does it ask for the IP address of the server?

##### Insert the IP address of the game server.

###### Does the client manage to connect to the server?

###### Does the client ask you for an username?

##### Insert your username.

###### Does the client initiate the graphical interface?

###### Are you presented with a mini map of the maze?

###### Can you see your position in the mini map?

###### When you move around in the world, does your position update in the mini map?

###### When you move around the maze, does the view of the camera update?

###### Do `W`, `A`, `S`, and `D` move the player forward, left, backward, and right?

###### Do the arrow keys control rotation, with left and right rotating counterclockwise and clockwise?

###### Does Space shoot a projectile?

###### Are movement and rotation speeds consistent at different frame rates?

###### Are walls and floors rendered on a consistent 32 x 32 pixel tile grid, with players, projectiles, and pickups using the documented standard sprite sizes?

###### Is the frame rate displayed in the interface?

###### Is the frame rate of the game higher than 50 fps?

##### Try to connect to the server from another computer.

###### Are you able to connect to the server? If you're forbidden from communicating between machines, this requirement may be fulfilled by demonstrating that the server accepts arbitrary IPs and that multiple clients can connect via `localhost`.

##### Connect simultaneously with as many people as possible and play the game for at least 3 minutes. Once again, if connecting multiple machines is not possible, running multiple local clients is also accepted.

###### Did the frame rate stay over 50 fps?

###### Independently of the frame rate displayed on the screen, does the game feel smooth?

##### Restart the server several times with different map seeds and inspect each generated map.

###### Is each playable map generated procedurally by the server, with its seed logged so the map can be reproduced?

###### Does the game provide at least 3 generated difficulty levels with progressively more dead ends or longer routes?

###### Is every player spawn on a walkable tile with at least two walkable neighboring tiles and enough clearance for the player collision bounds?

###### Can every spawn reach every other spawn and required objective through walkable tiles?

###### Are spawn points separated by the configured minimum path distance and protected from immediate direct firing lines?

###### When generation validation is deliberately made to fail, does the server retry a bounded number of times and then use a known-valid fallback rather than starting an invalid map?

#### Bonus

###### +Is it possible to edit your own maze?

###### +Can generated levels be reproduced from a recorded seed?

###### +Can you play against an A.I. player?

###### +Does the game initialization include a history of hosts with aliases for easier reconnection?
