#!/usr/bin/env python
#
# Design of wow modulation.
#
# Amplitude and phase of a sine wave is modulated using Ornstein-Uhlenbeck
# process, modeling brownian motion.
#
# Based on <https://github.com/mhampton/ZetaCarinaeModules>.

import math
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.widgets import Slider


def low_pass_filter(data, bandlimit, sample_rate):
    f = 2.0 * math.sin((math.pi * bandlimit) / sample_rate)
    q = 1.0 / 1.1
    delay_1 = 0.0
    delay_2 = 0.0

    filtered = np.zeros(len(data))

    for i in range(len(data)):
        sum_3 = delay_1 * f + delay_2
        sum_1 = data[i] - sum_3 - delay_1 * q
        sum_2 = sum_1 * f + delay_1
        delay_1 = sum_2
        delay_2 = sum_3
        filtered[i] = sum_3

    return filtered


def process_ornstein_uhlenbeck(random, mean, noise, spring, sample_rate):
    length = len(random)
    output = 0.0
    sqrt_delta = 1.0 / math.sqrt(sample_rate)

    processed = np.zeros(length)
    for i in range(length):
        output += spring * (mean[i] - output) * (1.0 / sample_rate)
        output += noise * random[i] * sqrt_delta
        processed[i] = output

    return processed


def generate_carrier(random, drift, frequency, sample_rate):
    length = len(random)
    carrier = np.zeros(length)
    phase = 1.5 * math.pi

    for i in range(length):
        carrier[i] = math.sin(phase)
        phase += 2.0 * math.pi * (frequency / sample_rate) * (1.0 + drift * random[i])

    return carrier / 2.0 + 0.5


slider_left = 0.05


def add_slider(fig, name, init, valmin, valmax):
    SLIDER_BOTTOM = 0.15
    SLIDER_WIDTH = 0.0225
    SLIDER_HEIGHT = 1.0 - SLIDER_BOTTOM * 2
    SLIDER_MARGIN = 0.03

    global slider_left
    slider = Slider(
        ax=fig.add_axes([slider_left, SLIDER_BOTTOM, SLIDER_WIDTH, SLIDER_HEIGHT]),
        label=name,
        valmin=valmin,
        valmax=valmax,
        valinit=init,
        orientation="vertical",
    )
    slider_left += SLIDER_MARGIN

    return slider


def plot():
    TIME = 2
    SAMPLE_RATE = 500
    INIT_FREQUENCY = 3.0
    INIT_AMPLITUDE = 0.3
    INIT_FILTER = SAMPLE_RATE / 12.0
    INIT_AMPLITUDE_NOISE = 0.5
    INIT_AMPLITUDE_SPRING = 40.0
    INIT_PHASE_NOISE = 1.0
    INIT_PHASE_SPRING = 1.0
    INIT_DRIFT = 0.3

    fig, ax = plt.subplots()
    ax.set_ylim([-0.02, 1.0])
    ax.grid(True, axis="y")

    fig.subplots_adjust(left=0.35)

    t = np.linspace(0, TIME, SAMPLE_RATE * TIME)
    random = np.random.rand(SAMPLE_RATE * TIME) * 2.0 - 1.0

    frequency_slider = add_slider(fig, "Freq", INIT_FREQUENCY, 0.1, 40.0)
    amplitude_slider = add_slider(fig, "Amp", INIT_AMPLITUDE, 0.0, 1.0)
    filter_slider = add_slider(fig, "Filter", INIT_FILTER, 0.0, SAMPLE_RATE * 0.2)
    amplitude_noise_slider = add_slider(fig, "ANoise", INIT_AMPLITUDE_NOISE, 0.0, 5.0)
    amplitude_spring_slider = add_slider(
        fig, "ASpring", INIT_AMPLITUDE_SPRING, 0.0, 300.0
    )
    phase_noise_slider = add_slider(fig, "PNoise", INIT_PHASE_NOISE, 0.0, 5.0)
    phase_spring_slider = add_slider(fig, "PSpring", INIT_PHASE_SPRING, 0.0, 10.0)
    drift_slider = add_slider(fig, "Drift", INIT_DRIFT, 0.0, 1.0)

    (line,) = ax.plot(np.zeros(SAMPLE_RATE * TIME))

    def update(_):
        phase_ou = process_ornstein_uhlenbeck(
            random,
            np.zeros(len(random)),
            phase_noise_slider.val,
            phase_spring_slider.val,
            SAMPLE_RATE,
        )
        carrier = (
            generate_carrier(
                phase_ou, drift_slider.val, frequency_slider.val, SAMPLE_RATE
            )
            * amplitude_slider.val
        )
        amplitude_ou = process_ornstein_uhlenbeck(
            random,
            carrier,
            amplitude_noise_slider.val,
            amplitude_spring_slider.val,
            SAMPLE_RATE,
        )
        wow = low_pass_filter(abs(amplitude_ou), filter_slider.val, SAMPLE_RATE)
        line.set_ydata(wow)
        fig.canvas.draw_idle()

    update(())

    frequency_slider.on_changed(update)
    amplitude_slider.on_changed(update)
    filter_slider.on_changed(update)
    amplitude_noise_slider.on_changed(update)
    amplitude_spring_slider.on_changed(update)
    phase_noise_slider.on_changed(update)
    phase_spring_slider.on_changed(update)
    drift_slider.on_changed(update)

    plt.show()


if __name__ == "__main__":
    plot()
