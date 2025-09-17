# ⚠️ WARNING ⚠️
This is a heavily WIP project that i work on when i want to/have free time. The code in most places here are half-baked spaghetti (like truly) written while learning everything from ground up (esp. the graphical stuff). So please be aware <3  

# Okay i understood that this is WIP project, but i want to learn more because i'm curious/interested
Great! So currently project is in "client works but a lot if not every feature is missing". Here's a list of what you can currently find/ not find:
* Actual hit-object osu! rendering (excluding spinners, follow lines, combo numbers, HUD stuff and etc)
* Gameplay processing. I wrote big chunk of gameplay processing completely from zero, so while "playing" you can receive actual judgments that are pretty in parity with stable (not lazer yet) . I'm trying to achieve parity through test coverage. Look into `tests/` directory to learn more. (spinners, note-lock, stacking are not here yet :))
* Song select menu. Build on top of `egui`, a lot of hacky stuff just to get some sort of layouting. There are really no other UI projects in Rust that can fulfill my requirements for this project besides `egui` so even it's hacky i'm probably gonna stick to it for the time being.
* Cross-platform. Thanks to the `winit` and `wgpu` it's probably runs on every platform but performance can degrade a lot depending on platform.
* ^ also runs on web, you can checkout it [here](https://rosu.lopij.xyz) but be prepared that it gonna eat 1GB of RAM and not run well
* Skin support, you can load skin through options (`Cntrl + O`) in song select menu
* Audio is not here at all, i'm not happy how `rodio` behaves so certainly gonna experiment in this field a lot

## Pretty cool. I want to run it locally just to try
Like every other Rust project clone repo and then just:
```
> cargo run --release
```
After you run client -> Select your osu!stable `Songs` folder/directory through options (`Cntrl + O`) -> `Import Beatmaps` to import some beatmaps, let it load a few beatmaps and then give it a try.
**Be prepare to catch some `panic!()`'s or completely broken stuff, you warned :)**

## What's the end goals for the project then?
Not gonna make big claims or anything. Currently my goal just to write my own osu!std client in my favourite programming language to sometimes be able to play it at evenings without any problems. And maybe learn something new in the process

## I'm super interested and wanna help in development/contribute
First of all i recommend you to give this though a few more tries :D  - there are not clear project structure, a lot of spaghetti code, a lot of prototyping stuff left in the project - do you really wanna deal with this? Anyways i'm always checking github feed or dm me at discord `@lopij` or find me at [BathBot](https://github.com/MaxOhn/Bathbot) discord (i'm not affiliated with `BathBot` in any sense it's just bade a really cool dude and there a lot of osu!rust devs in this server) - i'm always open to talk/approach

# Screenshots
<img width="2531" height="1429" alt="image" src="https://github.com/user-attachments/assets/e9981908-6794-4b5e-bdb7-c9d920095a2e" />
<img width="2555" height="1436" alt="image" src="https://github.com/user-attachments/assets/8f82402a-7838-4b0b-a0cb-cb89918db0a2" />
<img width="2528" height="1424" alt="image" src="https://github.com/user-attachments/assets/c389a3ec-0817-4f73-a2db-b210e1e1cc6b" />
