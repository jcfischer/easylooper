PlexLooper
==========

PlexLooper is a a VST audio plugin that allows for looping of audio material.
It's feature set is loosely based on the [Echoplex](http://www.loopers-delight.com/tools/echoplex/echoplex.html) looper 
by Gibson/Oberheim.

Currently only works on OSX due to the UI library being used. Tested in Ableton Live 10 and Reaper 9.5

Building
--------

    cargo build --release
    ./osx_vst_bundler.sh PlexLooper target/release/libplexlooper.dylib

Features
--------

* Record Loop
* Overdub (with Feedback)
* Mute
* Stop
* Replace (Replace parts of the loop with new material)
* Insert (Extend the loop with new material)
* Sync to subdivisions (Replace and Insert will start/stop at next subdivision of loop length)
* MIDI Control of above functions - currently hard coded to specific NoteOn/Off values
* Quantized replace: replace exactly the next subdivision with new material 
  [Quantized Replace](https://www.youtube.com/watch?v=g836XoN5plY&t=305s).
  Fails when stop event happens before the start event could fire (when syncing to very long subdivisions and 
  just tapping a short midi event)
  
Todo (roughly in order of priority)
-----------------------------------

* Correct handling of Feedback: on the Echoplex, the feedback control works during playback and reduces the amout of 
  signal in the recorded buffer
* Handle MIDI events in the correct order
* Smooth transistions between replaces/inserts to remove some of the glitching
* Quantize Modes (Off, Loop, Cycle, 8ths): and have the various functions respect the quantize mode
* Multiply: Extend the loop by repeating cycles [Multiply](https://www.youtube.com/watch?v=VmenN10KclQ)
* Unrounded Multiply: Extend or shorten the loop 
* New UI: maybe built with [mruby-zest](https://github.com/mruby-zest)
* Reverse
* Different speeds (at least 1/2 and double speeds). Use the [Rubberband](https://breakfastquay.com/rubberband/index.html)
  library
* Extreme time stretching (Non Echoplex function): Stretch time while keeping pitch (or dropping by one octave). 
  Create effects like [Paul Stretch](http://hypermammut.sourceforge.net/paulstretch/)  
* Configurable MIDI/OSC control
* Undo


Resources
---------

* [Echoplex Manual](http://aurisis.com/EchoplexPlusManual12.pdf)
* [Andre LaFosse Videos](https://www.youtube.com/playlist?list=PLRjhe9qWtn00cegswVPoUGSU-tQa07DQH)