# Game Design

## Central idea 
Sokoban game with rule discovery. The twist is that the only win condition is not being able to push any block.

## Game Elements
### Dynamic Elements
- Player
- Pushable block
- Pullable block
### Static Elements
- Wall
- Platform
- Pit

## Game Mechanics
- Player can move in cardinal directions
- Pushable blocks can be pushed by the player but not by other blocks
- Pullable blocks can only move if the player moves away in the opposite direction
- Walls are non-passable for all dynamic elements
- Pits are non-passable for the player but can be filled by a block, removing that block in the process, to make it passable
- Platforms are non-passable for dynamic elements, except for the player

## Red herrings 
The nature of having rule discovery with only a single rule calls for a lot of red herrings.
These have to be well designed and share something with the actual win condition.
Additionally if we make the player believe some rule that is extremely strict, we can try to put them into a situation where the only a relaxed version of the believed rule holds true. 
In this way we can incrementally break down these rigids rules to subsets that are more general until we arrive at the actual win condition.
This is largely inspired by the idea behind the [2-4-6 Task](https://en.wikipedia.org/wiki/Peter_Cathcart_Wason#Wason_and_the_2-4-6_Task).

### Examples
Here we will illustrate some relaxations of rules.
*Row*
- All blocks in the same row 
- All blocks are in the same row or column with at least another block 
- Some subset of all blocks are -""-
*Corner*
- All blocks in corner
- All blocks in corner or adjacent to block in corner
- Some subset of all blocks are -""-

## Unreachable
If a block is unreachable it will also be impossible to push the block. This is a rule that that can't be relaxed but it can be used consciously or not.
For example a level can be designed such that one (or more) blocks are unreachable from the start, this can lead to the player realizing that unreachable blocks are exempt from their current understanding of the rules. 
But one could also imagine a level with all blocks reachable by the player, but there is some configuration of blocks that trap the player such that they can't reach blocks outside the trap and such winning the game.



