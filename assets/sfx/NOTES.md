# GLINTBLADE sound effects

Written 2026-10-30.

## Palette and playback

The palette uses struck-glass partials, D major pentatonic crystal tones, airy blade sweeps, rounded impacts and a shared short sky-hall reverb.
Stone and fire provide weight; enemies dissolve into shards and light rather than gore.
The Warden introduction uses darker brass and stone beneath the same crystal palette.
All files are mono 16-bit PCM WAV at 44,100 Hz, normalized to -3 dBFS sample peak.
These are asset peaks, not intended playback volumes.
Suggested initial linear gains: blade 0.45, hits 0.5, steps 0.1, grass 0.12, UI tick 0.08, dialogue 0.06, heartbeat 0.15 and ambience 0.04 to 0.12.
Keep the heartbeat intermittent and fade it away when health recovers.
Use the longer stings once and lower the music during the introduction and finale.

Every filename is a loadable stem without its extension.
Some variant families start with an unnumbered base and then _2 and _3, while footsteps and blade swings start at _1.
Rotate only files present in the inventory.
Files ending in _loop are intended for looped playback, with a runtime gain fade when starting or stopping them.
The current game event path plays sounds once; new ambience and sustain assets require the caller to opt into looping.
Newly supplied effects are available assets; this sound-only pass does not wire new gameplay events.

## Production

The effects were synthesized in Python, with a few source layers generated through fal.ai.
This repository includes the finished WAV files.
The synthesis scripts, source layers and detailed audit results are not included.
The in-game mix still needs a human listening pass.

## Complete inventory

| File | Seconds | Use |
| --- | ---: | --- |
| amb_cave_loop.wav | 8.000 | Eight-second hollow cave air and distant drip bed |
| amb_lava_loop.wav | 8.000 | Eight-second molten rumble, bubbles and steam bed |
| amb_waterfall_loop.wav | 8.000 | Eight-second waterfall bed; gain follows distance |
| amb_wind_loop.wav | 8.000 | Eight-second sky-island wind bed |
| beam_charge.wav | 1.900 | Warden gathers light; build crests at 1.6 s |
| beam_fire.wav | 2.350 | Prism beam ignites and sweeps |
| beam_loop.wav | 1.000 | Continuous prism beam sustain; loop during longer sweeps |
| beetle_charge.wav | 0.800 | Crystal beetle scuttles into a charge |
| boss_crack.wav | 1.700 | Warden prism armor fractures |
| boss_die.wav | 6.500 | One-time finale: stone collapse and ascending pixel shower |
| boss_hit.wav | 1.000 | Sword strikes Warden crystal weak point |
| boss_intro.wav | 5.195 | One-time Warden awakening sting, before boss music |
| boss_roar.wav | 3.741 | Warden roar: resonant stone and prism energy |
| boss_shockwave.wav | 1.112 | Ground shockwave rolls away from the Warden |
| boss_slam.wav | 1.731 | Warden fist lands with stone impact and debris |
| boss_telegraph.wav | 0.600 | Warden attack warning |
| cave_gust.wav | 3.000 | Air moves through a distant cave tunnel |
| cave_pebbles.wav | 1.375 | Cave ceiling pebbles fall and skitter |
| cave_tone_1.wav | 3.546 | Distant large crystal resonates; play rarely |
| cave_tone_2.wav | 3.600 | Distant large crystal resonates; play rarely |
| charge.wav | 0.750 | Sword charge build, crest at about 0.47 s |
| charge_loop.wav | 1.000 | Held sword charge shimmer; loop while held |
| charge_ready.wav | 0.900 | Fully charged sword confirmation |
| crystal_shatter.wav | 0.794 | Crystal beetle or crystal prop breaks |
| crystal_shatter_2.wav | 0.850 | Crystal beetle or crystal prop breaks |
| dash.wav | 0.427 | Fast dodge with a crystalline afterimage |
| dash_2.wav | 0.408 | Fast dodge with a crystalline afterimage |
| deflect.wav | 0.450 | Blade deflects a projectile |
| drip_1.wav | 1.001 | Cave water drip with two echoes and a dark tail |
| drip_2.wav | 0.876 | Cave water drip with two echoes and a dark tail |
| drip_3.wav | 1.035 | Cave water drip with two echoes and a dark tail |
| ember_spit.wav | 0.426 | Totem spits a fireball |
| enemy_alert.wav | 0.248 | Enemy notices Glint |
| enemy_die.wav | 0.559 | Small enemy bursts into glowing pixels |
| enemy_die_2.wav | 0.653 | Small enemy bursts into glowing pixels |
| enemy_die_3.wav | 0.564 | Small enemy bursts into glowing pixels |
| enemy_windup.wav | 0.400 | Generic attack warning, before the attack fires |
| gate_locked.wav | 0.521 | Bump into a sealed gate |
| gate_open.wav | 3.092 | Crystal seal releases, stone gate slides and settles |
| gate_rumble.wav | 2.200 | Distant gate unseals; pair with slow camera shake |
| grass_cut.wav | 0.148 | Blade cuts a grass tuft; dry leafy texture |
| grass_cut_2.wav | 0.144 | Blade cuts a grass tuft; dry leafy texture |
| grass_cut_3.wav | 0.137 | Blade cuts a grass tuft; dry leafy texture |
| grass_rustle_1.wav | 0.240 | Brush through tall grass; rotate variants |
| grass_rustle_2.wav | 0.240 | Brush through tall grass; rotate variants |
| grass_rustle_3.wav | 0.240 | Brush through tall grass; rotate variants |
| heal.wav | 1.300 | Health restored: warm rise and crystal sparkle |
| heartbeat.wav | 0.343 | Low-health lub-dub warning; repeat about every 0.8 s |
| hit_1.wav | 0.178 | Blade connects with an enemy; rotate variants |
| hit_2.wav | 0.216 | Blade connects with an enemy; rotate variants |
| hit_3.wav | 0.182 | Blade connects with an enemy; rotate variants |
| hound_lunge.wav | 0.471 | Ember hound lunges with a fiery rush |
| lava_bubble_1.wav | 0.473 | Lava bubble breaks at the surface |
| lava_bubble_2.wav | 0.500 | Lava bubble breaks at the surface |
| lava_hiss_1.wav | 1.116 | Steam vent flares above lava |
| lava_hiss_2.wav | 0.626 | Debris lands in lava and sizzles |
| mote_pickup_1.wav | 0.440 | Healing mote absorbed; use numbered sequence for chains |
| mote_pickup_2.wav | 0.433 | Healing mote absorbed; use numbered sequence for chains |
| mote_pickup_3.wav | 0.397 | Healing mote absorbed; use numbered sequence for chains |
| player_die.wav | 2.538 | Glint falls: shattering energy and descending tones |
| player_hurt.wav | 0.331 | Glint takes damage: impact and falling energy |
| pot_break.wav | 0.421 | Clay pot breaks and fragments scatter |
| prism_shard.wav | 2.800 | Major region key item acquired; crystal bloom |
| proj_fizzle.wav | 0.196 | Projectile expires or dissipates |
| seed_pop.wav | 0.203 | Seed projectile lands and bursts |
| seed_throw.wav | 0.162 | Thorn imp throws a seed |
| shard_chime_1.wav | 2.400 | Nearby key shard calls; sparse positional ambience |
| shard_chime_2.wav | 2.153 | Nearby key shard calls; sparse positional ambience |
| shard_chime_3.wav | 2.183 | Nearby key shard calls; sparse positional ambience |
| shard_pickup.wav | 0.600 | Small shard pickup; numbered variants form a rising chain |
| shard_pickup_1.wav | 0.600 | Small shard pickup; numbered variants form a rising chain |
| shard_pickup_2.wav | 0.600 | Small shard pickup; numbered variants form a rising chain |
| shard_pickup_3.wav | 0.600 | Small shard pickup; numbered variants form a rising chain |
| shard_pickup_4.wav | 0.600 | Small shard pickup; numbered variants form a rising chain |
| shard_pickup_5.wav | 0.600 | Small shard pickup; numbered variants form a rising chain |
| slash_1.wav | 0.309 | First blade swing: airy sweep and short crystal ring |
| slash_2.wav | 0.321 | Combo backswing with a falling sweep |
| slash_3.wav | 0.800 | Heavy combo finisher with a shard flare |
| spin.wav | 1.100 | Charged spin: two passes and a crystal finish |
| sprout_hop.wav | 0.080 | Sproutling hops |
| step_grass_1.wav | 0.064 | Footstep on grass; rotate variants |
| step_grass_2.wav | 0.066 | Footstep on grass; rotate variants |
| step_grass_3.wav | 0.063 | Footstep on grass; rotate variants |
| step_stone_1.wav | 0.052 | Footstep on stone; rotate variants |
| step_stone_2.wav | 0.052 | Footstep on stone; rotate variants |
| step_stone_3.wav | 0.052 | Footstep on stone; rotate variants |
| step_water_1.wav | 0.167 | Footstep in shallow water; rotate variants |
| step_water_2.wav | 0.170 | Footstep in shallow water; rotate variants |
| step_water_3.wav | 0.170 | Footstep in shallow water; rotate variants |
| text_blip_1.wav | 0.070 | Dialogue text; rotate variants and skip some letters |
| text_blip_2.wav | 0.070 | Dialogue text; rotate variants and skip some letters |
| text_blip_3.wav | 0.070 | Dialogue text; rotate variants and skip some letters |
| ui_back.wav | 0.350 | Back out of a menu |
| ui_move.wav | 0.120 | Menu cursor movement |
| ui_select.wav | 0.500 | Confirm a menu choice |
| ui_start.wav | 1.657 | Begin the adventure from the title screen |
| ui_tick.wav | 0.065 | Soft focus or toggle tick; use low playback gain |
| wisp_shot.wav | 0.420 | Wisp fires a glowing magic bolt |
