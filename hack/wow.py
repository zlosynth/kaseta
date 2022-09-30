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
    bandlimit_index = int(bandlimit * data.size / sample_rate)

    data_frequency_domain = np.fft.fft(data)

    for i in range(bandlimit_index + 1, len(data_frequency_domain) - bandlimit_index):
        data_frequency_domain[i] = 0

    data_filtered = np.fft.ifft(data_frequency_domain)

    return np.real(data_filtered)


def process_ornstein_uhlenbeck(random, noise, spring, sample_rate):
    MEAN = 0.0
    length = len(random)
    output = 0.0
    sqrt_delta = 1.0 / math.sqrt(sample_rate)

    processed = np.zeros(length)
    for i in range(length):
        output += spring * (MEAN - output) * (1.0 / sample_rate)
        output += noise * random[i] * sqrt_delta
        processed[i] = output

    return processed


def generate_carrier(random, drift, frequency, sample_rate):
    length = len(random)
    carrier = np.zeros(length)
    phase = 0.0

    for i in range(length):
        carrier[i] = math.sin(phase)
        phase += 2.0 * math.pi * (frequency / sample_rate) * (1.0 + drift * random[i])

    return carrier


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
    SAMPLE_RATE = 1000
    INIT_AMPLITUDE_NOISE = 1.0
    INIT_AMPLITUDE_SPRING = 1.0
    INIT_AMPLITUDE_FILTER = SAMPLE_RATE / 12.0
    INIT_PHASE_NOISE = 1.0
    INIT_PHASE_SPRING = 1.0
    INIT_PHASE_FILTER = SAMPLE_RATE / 2.0
    INIT_DRIFT = 0.3
    INIT_FREQUENCY = 10.0
    INIT_AMPLITUDE = 0.3

    fig, ax = plt.subplots()
    ax.set_ylim([-1.0, 1.0])

    fig.subplots_adjust(left=0.35)

    t = np.linspace(0, 1, SAMPLE_RATE)
    random = np.random.rand(SAMPLE_RATE) * 2.0 - 1.0

    amplitude_noise_slider = add_slider(fig, "ANoise", INIT_AMPLITUDE_NOISE, 0.0, 5.0)
    amplitude_spring_slider = add_slider(
        fig, "ASpring", INIT_AMPLITUDE_SPRING, 0.0, 10.0
    )
    amplitude_filter_slider = add_slider(
        fig, "AFilter", INIT_AMPLITUDE_FILTER, 0.0, SAMPLE_RATE / 2
    )
    phase_noise_slider = add_slider(fig, "PNoise", INIT_PHASE_NOISE, 0.0, 5.0)
    phase_spring_slider = add_slider(fig, "PSpring", INIT_PHASE_SPRING, 0.0, 10.0)
    phase_filter_slider = add_slider(
        fig, "PFilter", INIT_PHASE_FILTER, 1.0, SAMPLE_RATE / 2
    )
    drift_slider = add_slider(fig, "Drift", INIT_DRIFT, 0.0, 1.0)
    frequency_slider = add_slider(fig, "Freq", INIT_FREQUENCY, 0.1, 40.0)
    amplitude_slider = add_slider(fig, "Amp", INIT_AMPLITUDE, 0.0, 1.0)

    (line,) = ax.plot(np.zeros(SAMPLE_RATE))

    def update(_):
        amplitude_ou = low_pass_filter(
            process_ornstein_uhlenbeck(
                random,
                amplitude_noise_slider.val,
                amplitude_spring_slider.val,
                SAMPLE_RATE,
            ),
            amplitude_filter_slider.val,
            SAMPLE_RATE,
        )
        phase_ou = low_pass_filter(
            process_ornstein_uhlenbeck(
                random,
                phase_noise_slider.val,
                phase_spring_slider.val,
                SAMPLE_RATE,
            ),
            phase_filter_slider.val,
            SAMPLE_RATE,
        )
        carrier = (
            generate_carrier(
                phase_ou, drift_slider.val, frequency_slider.val, SAMPLE_RATE
            )
            * amplitude_slider.val
        )
        line.set_ydata(amplitude_ou + carrier)
        fig.canvas.draw_idle()

    update(())

    amplitude_noise_slider.on_changed(update)
    amplitude_spring_slider.on_changed(update)
    amplitude_filter_slider.on_changed(update)
    phase_noise_slider.on_changed(update)
    phase_spring_slider.on_changed(update)
    phase_filter_slider.on_changed(update)
    drift_slider.on_changed(update)
    frequency_slider.on_changed(update)
    amplitude_slider.on_changed(update)

    plt.show()


if __name__ == "__main__":
    plot()
