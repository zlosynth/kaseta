# Changelog

All notable changes to this project will be documented in this file. See
[VERSIONING.md](VERSIONING.md) for more information about versioning and
backwards compatibility.

## Unreleased

* Add more embedded tests.
* Fix I/O bugs found while testing with a prototype.
* Finalize user manual's front page.
* Trigger flutter in sporadic pulses.
* Optimize wow and flutter algorithms.

## 0.5.0

* Further refactoring of the control module.
* Control speed through clock signal or tap-in.
* Implement firmware binding.
* Recover configuration after restart.
* Introduce internal oscillator.
* Trigger impulse by a chance set through volume.
* Optionally apply tone only on feedback.
* Apply DC blocking filter on output.
* Allow configuration of rewind speeds per head.

## 0.4.0

* Design the PCB.
* Initialize basic skeleton of firmware.
* Implement stereo output and panning.
* Introduce board peripherals abstraction.
* Blink status LED.
* Implement input calibration.
* Allow mapping of the CV input to an arbitrary attribute.
* Provide a configuration menu.
* Detect tempo from tap on button or clock in CV.
* Completelly rework control manipulation.

## 0.3.0

* Introduce tone control.
* Tweak wow response and control it using a single attribute.
* Implement basic flutter as an alternative to wow.
* Employ linear interpolation for wow and flutter.

## 0.2.0

* Extend maximum delay size to 2 minutes.
* Introduce delay impulse output, turning Kaseta into a trigger sequencer.

## 0.1.0

* Create a proof of concept, providing basic support of saturation, delay,
  wow and flutter.
* Design basic outline of the PCB.
* Prepare a draft of the user manual.
