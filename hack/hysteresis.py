#!/usr/bin/env python
#
# Kudos to Jatin Chowdhury
# * https://jatinchowdhury18.medium.com/complex-nonlinearities-episode-3-hysteresis-fdeb2cd3e3f6
# * https://dafx2019.bcu.ac.uk/papers/DAFx2019_paper_3.pdf
# * https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf
# * https://github.com/jatinchowdhury18/audio_dspy

from csv import DictWriter
import argparse
import sys
import concurrent.futures

from scipy.optimize import curve_fit
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

FS = 48000 * 8


class Differentiator:
    """Time domain differentiation using the trapezoidal rule"""

    def __init__(self, fs):
        self.T = 1.0 / fs
        self.x_1 = 0.0
        self.x_d_n1 = 0.0

    def differentiate(self, x):
        d_alpha = 0.75
        x_d = (((1 + d_alpha) / self.T) * (x - self.x_1)) - d_alpha * self.x_d_n1
        self.x_1 = x
        self.x_d_n1 = x_d
        return x_d


class Hysteresis:
    """Class to implement hysteresis processing"""

    def __init__(self, drive, saturation, width, fs):
        """
        Parameters
        ----------
        drive : float
            Hysteresis drive parameter
        saturation : float
            Saturation parameter
        width : float
            Hysteresis width parameter
        fs : float
            Sample rate
        """
        self.deriv = Differentiator(fs)
        self.T = 1.0 / fs
        self.M_s = 0.5 + 1.5 * (1 - saturation)  # saturation
        self.a = self.M_s / (0.01 + 6 * drive)  # adjustable parameter
        self.alpha = 1.6e-3
        self.k = 30 * (1 - 0.5) ** 6 + 0.01  # coercivity
        self.c = (1 - width) ** 0.5 - 0.01  # changes slope

    @staticmethod
    def langevin(x):
        """Langevin function: coth(x) - (1/x)"""
        if abs(x) > 10**-4:
            return (1 / np.tanh(x)) - (1 / x)
        else:
            return x / 3

    @staticmethod
    def langevin_deriv(x):
        """Derivative of the Langevin function: (1/x^2) - coth(x)^2 + 1"""
        if abs(x) > 10**-4:
            return (1 / x**2) - (1 / np.tanh(x)) ** 2 + 1
        else:
            return 1 / 3

    def dMdt(self, M, H, H_d):
        """Jiles-Atherton differential equation

        Parameters
        ----------
        M : float
            Magnetisation
        H : float
            Magnetic field
        H_d : float
            Time derivative of magnetic field

        Returns
        -------
        dMdt : float
            Derivative of magnetisation w.r.t time
        """
        Q = (H + self.alpha * M) / self.a
        M_diff = self.M_s * self.langevin(Q) - M
        delta_S = 1 if H_d > 0 else -1
        delta_M = 1 if np.sign(delta_S) == np.sign(M_diff) else 0
        L_prime = self.langevin_deriv(Q)

        denominator = 1 - self.c * self.alpha * (self.M_s / self.a) * L_prime

        t1_num = (1 - self.c) * delta_M * M_diff
        t1_den = (1 - self.c) * delta_S * self.k - self.alpha * M_diff
        t1 = (t1_num / t1_den) * H_d

        t2 = self.c * (self.M_s / self.a) * H_d * L_prime

        return (t1 + t2) / denominator

    def RK4(self, M_n1, H, H_n1, H_d, H_d_n1):
        """Compute hysteresis function with Runge-Kutta 4th order

        Parameters
        ----------
        M_n1 : float
            Previous magnetisation
        H : float
            Magnetic field
        H_n1 : float
            Previous magnetic field
        H_d : float
            Magnetic field derivative
        H_d_n1 : float
            Previous magnetic field derivative

        Returns
        -------
        M : float
            Current magnetisation
        """
        k1 = self.T * self.dMdt(M_n1, H_n1, H_d_n1)
        k2 = self.T * self.dMdt(M_n1 + k1 / 2, (H + H_n1) / 2, (H_d + H_d_n1) / 2)
        k3 = self.T * self.dMdt(M_n1 + k2 / 2, (H + H_n1) / 2, (H_d + H_d_n1) / 2)
        k4 = self.T * self.dMdt(M_n1 + k3, H, H_d)
        return M_n1 + (k1 / 6) + (k2 / 3) + (k3 / 3) + (k4 / 6)

    def process_block(self, x):
        """Process block of samples"""
        M_out = np.zeros(len(x))
        M_n1 = 0
        H_n1 = 0
        H_d_n1 = 0

        n = 0
        for H in x:
            H_d = self.deriv.differentiate(H)
            M = self.RK4(M_n1, H, H_n1, H_d, H_d_n1)

            M_n1 = M
            H_n1 = H
            H_d_n1 = H_d

            M_out[n] = M
            n += 1

        return M_out


def generate_sine(frequency, length):
    time = np.linspace(0, length, int(length * FS))
    return np.sin(frequency * 2 * np.pi * time)


def plot_harmonic_response(ax, signal):
    N = len(signal)

    Y = np.fft.rfft(signal)
    Y = Y / np.max(np.abs(Y))

    f = np.linspace(0, FS / 2, int(N / 2 + 1))

    ax.semilogx(f, 20 * np.log10(np.abs(Y)))
    ax.set_xlim([20, 20000])
    ax.set_ylim([-90, 5])


def plot_hysteresis_loop(ax, original, processed):
    ax.plot(original, processed)


def plot_signal(ax, time, signal):
    ax.plot(time, signal)


def analyze_processor(axs, column, processor, attributes):
    FREQUENCY = 100
    LENGTH = 0.08

    ax_loop = axs[0, column]
    ax_signal = axs[1, column]
    ax_response = axs[2, column]

    legend = []

    # plot only the second half, after hysteresis stabilizes
    signal = generate_sine(FREQUENCY, length=LENGTH)
    half = int(len(signal) / 2)
    half_signal = signal[half:]
    time = np.linspace(0, LENGTH, int(FS * LENGTH))
    half_time = time[:half]

    for a in attributes:
        processed = processor(signal, a)
        half_processed = processed[half:]
        plot_hysteresis_loop(ax_loop, half_signal, half_processed)
        plot_signal(ax_signal, half_time, half_processed)
        plot_harmonic_response(ax_response, half_processed)
        legend.append(a)

    ax_loop.legend(legend, loc="upper center", bbox_to_anchor=(0.5, 1.4), ncol=3)

    plot_signal(ax_signal, half_time, half_signal)


def response():
    fig, axs = plt.subplots(3, 3)

    axs[0, 0].set_ylabel("Hysteresis loop")
    axs[1, 0].set_ylabel("Processed signal")
    axs[2, 0].set_ylabel("Harmonic response")

    def processor(
        block,
        drive=1.0,
        saturation=0.9,
        width=1.0,
    ):
        return Hysteresis(drive, saturation, width, FS).process_block(block)

    axs[0, 0].set_title("Drive")
    analyze_processor(
        axs,
        0,
        lambda block, drive: processor(block, drive=drive),
        [0.0, 0.1, 0.25, 0.5, 1.0, 5.0, 10.0, 20.0],
    )

    axs[0, 1].set_title("Saturation")
    analyze_processor(
        axs,
        1,
        lambda block, saturation: processor(block, saturation=saturation),
        [0.0, 0.5, 1.0],
    )

    axs[0, 2].set_title("Width")
    analyze_processor(
        axs,
        2,
        lambda block, width: processor(block, width=width),
        [0.0, 0.5, 0.99],
    )

    plt.show()


def amplitude_generate():
    I = 10
    D = 20
    S = 20
    W = 20

    input_configs = []
    for i in np.linspace(0.2, 1.0, I):
        for d in np.linspace(0.2, 20.0, D):
            for s in np.linspace(0, 1, S):
                for w in np.linspace(0, 0.9999, W):
                    input_configs.append(
                        {
                            "i": i,
                            "d": d,
                            "s": s,
                            "w": w,
                        }
                    )

    i = 1
    m = I * D * S * W
    configs = []
    with concurrent.futures.ProcessPoolExecutor() as executor:
        for config in executor.map(set_max_amplitude, input_configs):
            configs.append(config)
            print(f"{i}/{m}")
            i += 1

    with open("amplitude_dataset.csv", "w", newline="") as f:
        writer = DictWriter(f, fieldnames=configs[0].keys())
        writer.writeheader()
        writer.writerows(configs)

    print("Done")


def set_max_amplitude(config):
    FREQUENCY = 100.0
    LENGTH = 0.02
    signal = generate_sine(FREQUENCY, length=LENGTH) * config["i"]
    config["a"] = np.max(
        Hysteresis(config["d"], config["s"], config["w"], FS).process_block(signal)
    )
    return config


if __name__ == "__main__":
    parser = argparse.ArgumentParser(prog=sys.argv[0])
    subparsers = parser.add_subparsers(
        help="sub-command help", required=True, dest="subparser"
    )
    subparsers.add_parser(
        "response", help="Plot processed signal, hysteresis loop, and harmonic response"
    )
    subparsers.add_parser(
        "amplitude_generate",
        help="Generate dataset mapping input arguments to amplitude",
    )
    args = parser.parse_args()

    if args.subparser == "response":
        response()
    elif args.subparser == "amplitude_generate":
        amplitude_generate()
