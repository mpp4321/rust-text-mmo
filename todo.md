* Commands
  - Need to list actions for an object when looking at it

* Persistent client data based on username/password login when a client connects
 - Will need a way to calculate offline gains eventually;
 - ^ offline / game connected features will need some type of scripting language interop

* Need actual game features of some sort
  * Inventory.
    - how can we make it so players cannot abuse item creation too?
    - possibly functional items are completely RNG?
      * names, descriptions + stats
      * when creating a item drop for something
        - allows specification of name params whether its RNG
        - allow specification of description(s)
        - 1 stat for now per item: "difficulty of enemy" + "level of enemy" + (rng(0, level//5))
    - allow creation of arbitrary misc items
    - ^ for user story creation purposes
  * Levels? Skills (infinite skills?)?
   - Skills defined by players; action -> xp in skill -> gain skill if you dont have it and xp 
   - Action requires some arbitrary skill defined by String key -
   - so players cannot abuse xp gains by just adding objects which give tons of xp
     - xp gains based on couple of parameters which are relative
     - ^ "difficulty of action" * "level requirement"; both which can be set by creator of object
     - ^
