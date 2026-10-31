# GLINTBLADE soundtrack

Finished 2026-10-30.

Six original generated cues, selected from the saved candidates using anonymized FAL audio-understanding descriptions.
Descriptions below are model listening reports, not claims of human audition.
The intended arc is open-sky adventure, cold underground tension, volcanic danger, then a larger final confrontation.

All files are OGG Vorbis, 44,100 Hz stereo, normalized using ffmpeg loudnorm measurements and one static gain per track.
Static gain preserves the circular crossfades; dynamic gain riding across the file boundary is avoided.
Verification decodes the delivered OGGs with ffmpeg and checks integrated loudness, true peak, duration, channels, sample count, and boundary continuity.

| File | Duration | Decoded loudness | True peak | Playback |
| --- | ---: | ---: | ---: | --- |
| title.ogg | 69.354 s | -16.05 LUFS | -4.26 dBTP | Loop |
| bloomwild.ogg | 69.840 s | -16.03 LUFS | -4.29 dBTP | Loop |
| glasswater.ogg | 69.818 s | -15.99 LUFS | -2.80 dBTP | Loop |
| cinder.ogg | 86.919 s | -16.03 LUFS | -2.88 dBTP | Loop |
| warden.ogg | 65.143 s | -16.04 LUFS | -4.34 dBTP | Loop |
| victory.ogg | 8.000 s | -16.09 LUFS | -4.89 dBTP | One-shot |

## title

Model: `google/lyria-3.5`.
Source: `tools/music/candidates/r3tt_l35_a.mp3` (candidate `r3tt_l35_a`).

Loop method: cut 23.95000 to 93.30361 seconds on detected beat/bar boundaries, with sample-level transient alignment.
A 1.94-second equal-power crossfade uses the matching preceding bar before the head at the tail.
Decoded wrap sample jump is 0.016831; the track's 99.9th-percentile sample step is 0.133981.

Generation prompt:

> Instrumental music for a pixel-art action-adventure game, the title screen: a small hooded swordsman stands on the edge of a floating sky island at dawn, looking up at a cracked crystal prism at the top of the world.
> Genre: cosmic Berlin School analog-synth score, 1970s and 80s style.
> 92 BPM, A minor.
> Instruments: a hypnotic pulsing analog sequencer bass, huge detuned CS-80 style brass-synth chords swelling like a sunrise, warm tape-saturated pads with gentle wow and flutter, a glassy synth lead with a memorable, wistful but heroic melody, deep floor toms and a gated snare that drive the second half forward.
> Mysterious, wide and adventurous: the feeling of a journey about to begin.
> Long dark reverb.
> No vocals, no choir.
> Length: about 2 minutes.

Final-file blind listener (`track_29c23c06.mp3`, entire 69.35-second file):

> **Genre:** Orchestral Synthwave / Uplifting Trance
> **Instruments:** Lead synthesizers, arpeggiators, synth pads, drum machine, prominent synthetic choir.
> **Tempo and energy:** Mid-tempo with consistently high, soaring energy that builds throughout.
> **Mood:** Epic, hopeful, and adventurous, conveying a strong sense of wonder.
> **Sounds like:** The *Mass Effect* galaxy map, Nobuo Uematsu's synth work, M83.
> **Fits:** A title screen for a sci-fi RPG, or exploring a vibrant, high-tech city or celestial-themed overworld.
> **Stock music:** Somewhat.
> The professional production is paired with a conventional structure and common synth presets, making it sound like high-quality library music rather than a bespoke main theme.
> **Vocals:** Synthetic choir pads are prominent; no actual human voice, chops, or lyrics.
> **Flaws:** None.
> The mix is clean, well-balanced, and free of artifacts or distortion.

Candidate listening record: `r3tt_l35_a` / `track_2460bc68.mp3` in `tools/music/listen/results.json`.

Separate blind loop-boundary check (`track_06f8eda6.mp3`, wrap at 8 seconds):

> This is an 80s-style synth-pop track featuring a driving drum machine and layered synthesizers.
> 
> The editing continuity is seamless.
> There are no audible clicks, gaps, double drum hits, or unintended loudness dips.
> The prominent transition around 8 seconds is a deliberate musical change, marking a new section with a drum fill and a new synth layer.
> It is a well-executed compositional shift, not a jarring edit or harmonic flaw.
> The tempo is stable throughout.

## bloomwild

Model: `google/lyria-3.5`.
Source: `tools/music/candidates/r3bw_l35_a.mp3` (candidate `r3bw_l35_a`).

Loop method: cut 11.94000 to 81.77984 seconds on detected beat/bar boundaries, with sample-level transient alignment.
A 0.18-second equal-power crossfade uses the matching tail continuation at the head.
Decoded wrap sample jump is 0.066426; the track's 99.9th-percentile sample step is 0.124813.

Generation prompt:

> Instrumental music for a pixel-art action-adventure game: a tiny hooded swordsman dashing through overgrown white-stone ruins on a floating sky island at golden hour, waterfalls pouring into a sea of clouds, petals in the wind.
> Genre: upbeat motorik krautrock with vintage analog synths.
> 132 BPM, D major with a bright Lydian color.
> Instruments: a relentless motorik beat on a real drum kit, a driving eighth-note bass guitar, bubbling analog synth arpeggios, a chiming clean electric guitar riff, a playful kalimba counter-melody, and a soaring detuned analog synth lead with a hummable, daring hook.
> Sunny, energetic and curious, like running into the wind.
> Warm tape saturation, airy reverb.
> Full groove from the first bar, no fade-out.
> No vocals.
> Length: about 2 minutes.

Final-file blind listener (`track_2e43ab85.mp3`, entire 69.84-second file):

> **Genre:** J-pop instrumental / Video Game Music (VGM)
> **Instruments:** Drum machine, synth bass, layered synth leads, arpeggiated synths, pads.
> **Tempo and energy:** Fast and high-energy, relentlessly driving and danceable.
> **Mood:** Exuberant, joyful, optimistic, and celebratory.
> **Sounds like:** *Touhou Project* soundtrack, *Ys* series (modern arrangements), Japanese rhythm games.
> **Fits:** A high-speed racing level in a futuristic city, a celebratory results screen, or an anime-style RPG opening sequence.
> **Stock music:** Somewhat.
> It's too well-composed for generic corporate use but is a perfect archetype for a "royalty-free J-pop" library.
> **Vocals:** None.
> **Flaws:** None.
> The mix is clean, punchy, and well-balanced.

Candidate listening record: `r3bw_l35_a` / `track_02328fe2.mp3` in `tools/music/listen/results.json`.

Separate blind loop-boundary check (`track_07e609fc.mp3`, wrap at 8 seconds):

> This is an upbeat, high-energy J-rock or video game-style track with fast synth melodies and a driving drum beat.
> 
> The rhythm and harmony flow naturally and cohesively throughout the clip.
> The music is intentionally busy with rapid drum fills and melodic runs, but it is musically coherent.
> There are no audible editing clicks, gaps, stutters, doubled drum hits, sudden volume dips, or awkward harmonic jumps.
> The production sounds clean and well-executed, with all elements tightly integrated.

## glasswater

Model: `google/lyria-3.5`.
Source: `tools/music/candidates/r5gw_l35.mp3` (candidate `r5gw_l35`).

Loop method: cut 39.38000 to 109.19816 seconds on detected beat/bar boundaries, with sample-level transient alignment.
A 2.18-second equal-power crossfade uses the matching tail continuation at the head.
Decoded wrap sample jump is 0.002840; the track's 99.9th-percentile sample step is 0.081906.

Generation prompt:

> Cold, glittering darkwave with a minimal techno pulse: a ticking 16th-note analog sequencer bass, a tight dry drum machine with rimshots and shuffled hi-hats, crystalline FM-bell and glass-harmonica arpeggios ringing in a huge cave reverb, slow dark pads, water-drop plucks bouncing on a ping-pong delay, and a sparse, eerie synth lead.
> 112 BPM, C sharp minor.
> Scene: wading through shallow water in deep indigo caverns lit by giant glowing cyan and violet crystals.
> Mysterious, tense and curious, always moving forward.
> Instrumental, no vocals.
> Length: about 2 minutes.

Final-file blind listener (`track_050fe235.mp3`, entire 69.82-second file):

> **Genre:** Cinematic Cyberpunk
> **Instruments:** Synthesizers (leads, bass, arpeggios), electronic drums, digital strings.
> **Tempo and energy:** A driving, mid-tempo beat with high, building energy that creates a sense of urgency.
> **Mood:** Dramatic, tense, futuristic, and determined.
> **Sounds like:** *Cyberpunk 2077*, *Mass Effect*.
> **Fits:** A high-stakes vehicle chase through a neon-lit city or gearing up for a critical story mission.
> **Stock music:** Somewhat.
> While the production quality is high, the structure and melodic progressions feel formulaic, like premium royalty-free music designed for "epic sci-fi" montages.
> **Vocals:** None.
> **Flaws:** None.
> The mix is clean and professional with good separation.

Candidate listening record: `finish_gw5` / `track_0bff40f9.mp3` in `tools/music/listen/results.json`.

Separate blind loop-boundary check (`track_cc11542c.mp3`, wrap at 8 seconds):

> This is an epic, synth-driven electronic track with a driving beat.
> The editing continuity is excellent, exhibiting professional production.
> There are no audible clicks, gaps, unintentional double hits, or sudden loudness dips.
> 
> The notable event near 8 seconds is a deliberate and stylistic rhythmic stutter/glitch effect, a common production technique in this genre used to create interest.
> This is not an editing flaw, an abrupt tempo change, or a jarring harmonic jump; all musical changes throughout the clip sound intentional and are smoothly executed.

## cinder

Model: `google/lyria-3.5`.
Source: `tools/music/candidates/r4csA_l35.mp3` (candidate `r4csA_l35`).

Loop method: cut 9.11000 to 96.02937 seconds on detected beat/bar boundaries, with sample-level transient alignment.
A 2.18-second equal-power crossfade uses the matching tail continuation at the head.
Decoded wrap sample jump is 0.013795; the track's 99.9th-percentile sample step is 0.144547.

Generation prompt:

> Darksynth: a menacing distorted analog bass arpeggio, a pounding gated drum machine, horror-film brass-synth stabs, screaming sawtooth leads and molten low drones.
> 112 BPM, E Phrygian.
> Scene: black obsidian ruins and rivers of glowing lava under a purple and orange sky, embers rising.
> Hot, dangerous and swaggering, like a chase through fire.
> Instrumental, no vocals.
> Length: about 2 minutes.

Final-file blind listener (`track_337e944e.mp3`, entire 86.92-second file):

> **Genre:** Aggressive Synthwave / Cyberpunk
> **Instruments:** Heavy synthesizers (bass, leads, arps), powerful electronic drums.
> **Tempo and energy:** Fast-paced and high-energy; relentless and driving.
> **Mood:** Intense, aggressive, futuristic, and action-oriented.
> **Sounds like:** *Cyberpunk 2077*, Mick Gordon, Perturbator.
> **Fits:** A frantic boss battle in a sci-fi shooter, a high-speed chase through a neon-drenched city, or an arena combat montage.
> **Stock music:** Somewhat.
> The production is clean and energetic, but the structure is a predictable build-and-drop formula with synth sounds that feel common to royalty-free libraries.
> **Vocals:** None.
> **Flaws:** None.
> The mix is loud and compressed by design, but clean.

Candidate listening record: `finish_csA` / `track_b1ca5612.mp3` in `tools/music/listen/results.json`.

Separate blind loop-boundary check (`track_208ece22.mp3`, wrap at 8 seconds):

> This is a driving, cinematic synthwave track with a tense, futuristic feel.
> The editing continuity is excellent; there are no audible clicks, gaps, unintentional double drum hits, or sudden loudness dips.
> 
> The abrupt stop-and-start transition near 8 seconds is a deliberate, musically effective hard cut, not an editing flaw.
> It is a compositional choice to introduce a new, more intense section with a percussive impact, rather than a jarring harmonic jump or a tempo error.
> The overall production is clean and professional.

## warden

Model: `google/lyria-3.5`.
Source: `tools/music/candidates/r5wd_l35.mp3` (candidate `r5wd_l35`).

Loop method: cut 37.79000 to 102.93281 seconds on detected beat/bar boundaries, with sample-level transient alignment.
A 1.72-second equal-power crossfade uses the matching tail continuation at the head.
Decoded wrap sample jump is 0.035959; the track's 99.9th-percentile sample step is 0.114637.

Generation prompt:

> Cosmic darksynth with a cathedral pipe organ: a massive organ riff in D minor, a driving sequenced analog bass, pounding live drums with double kick and huge tom fills, slow crushing brass-synth blasts like the footsteps of a stone giant, a glittering arpeggiator like a sky full of stars, and a screaming analog lead.
> 144 BPM.
> Scene: facing a towering ancient stone guardian with a cracked, glowing crystal heart on a mountaintop under stars and aurora.
> Urgent, dangerous and exhilarating, larger than life but fun.
> Instrumental, no vocals, no choir.
> Length: about 2 minutes.

Final-file blind listener (`track_06381d28.mp3`, entire 65.14-second file):

> Genre: Symphonic J-Rock / Video Game Metal
> Instruments: Distorted electric guitars, fast drums, driving bass, synth leads, orchestral strings.
> Tempo and energy: Extremely fast and high-energy.
> A relentless, full-throttle sonic assault.
> Mood: Epic, intense, chaotic, and urgent.
> An adrenaline-pumping battle anthem.
> Sounds like: Daisuke Ishiwatari (*Guilty Gear*), Falcom Sound Team jdk (*Ys*), *Devil May Cry 5*.
> Fits: The climactic final boss battle in a fast-paced JRPG or action fighting game.
> Stock music: No.
> The composition is far too complex, dynamic, and stylistically distinct.
> It has a clear, professional artistic identity.
> Vocals: None.
> Flaws: None.
> The very loud, dense mix is a deliberate stylistic choice for the genre and is well-executed.

Candidate listening record: `finish_wd5` / `track_31b2bbe4.mp3` in `tools/music/listen/results.json`.

Separate blind loop-boundary check (`track_59afd792.mp3`, wrap at 8 seconds):

> This is an intense, high-energy piece blending symphonic metal and video game music, with fast guitar riffs and complex drumming.
> 
> The editing continuity is excellent.
> No audible clicks, gaps, or unintended double drum hits were detected.
> The transition near 8 seconds features an intentional, virtuosic drum fill and a rapid harmonic shift, which are hallmarks of this musical style.
> This is a deliberate, high-impact musical change, not an editing flaw.
> Loudness and tempo remain consistent and controlled throughout, with no jarring artifacts or dips present in the recording.

## victory

Model: `elevenlabs/music/v2.5`.
Source: `tools/music/candidates/finish_vic_el_b.mp3` (candidate `finish_vic_el_b`).

One-shot method: retain 0.000 to 8.000 seconds, with a 0.005-second de-click fade-in and a 1.20-second final decay fade.
Play once after the Warden falls; do not loop.

Generation prompt:

> 3 seconds: Instrumental synth-brass victory fanfare, fortissimo, explosive joyful arrival; Three bold ascending brass-synth chord stabs in D major, a rapid ascending crystal-bell flourish, a deep tom roll and one crash cymbal; One concise dramatic victory flourish, immediately celebratory, 144 BPM
> Excluded: vocals, choir, ambient, looping music, gentle background music.
>
> 5 seconds: Land on one big triumphant sustained D major add9 chord played by warm analog brass synthesizers and sparkling crystal bells; Let this single chord decay into complete silence, long shimmering reverb tail; Instrumental single final chord, no continuing beat
> Excluded: vocals, choir, loop, new melody, beat, fade into another section.
>

Final-file blind listener (`track_41903b8a.mp3`, entire 8.00-second file):

> Genre: Triumphant Fanfare
> Instruments: Orchestral brass (trumpets, horns), strings, timpani.
> Tempo and energy: Upbeat and high-energy; a powerful, celebratory burst.
> Mood: Heroic, victorious, grand, and uplifting.
> Sounds like: The victory themes from *Final Fantasy*, the castle music from *Super Mario 64*, a classic Disney film opening.
> Fits: A game's victory screen, level completion, award ceremony, or the main title sequence for a fantasy adventure.
> Stock music: Somewhat.
> It's a clean, high-quality execution of a common "heroic fanfare" trope, making it sound like premium library music.
> Vocals: None.
> Flaws: None.

Candidate listening record: `finish_victory2` / `track_88bd11db.mp3` in `tools/music/listen/results.json`.

## Source files

The finished tracks are included in this repository.
The source audio, listening logs and build scripts referenced above are not included.
