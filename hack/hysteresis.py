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
AMPLITUDE_DATASET = "amplitude_dataset.csv"


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

    def __init__(self, drive, saturation, width, fs, makeup=False):
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
        if makeup:
            self.makeup = Hysteresis.makeup(drive, saturation, width)
        else:
            self.makeup = 1

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

            M_out[n] = M * self.makeup
            n += 1

        return M_out

    def makeup(drive, saturation, width):
        d = max(drive, 0.1)
        s = saturation
        w = width

        # func_j_pow2
        a1 = 1.3679276126999933
        a2 = 0.912466149478303
        a3 = -1.4378610485082859
        a4 = 1.1241058501596586
        a5 = -0.9857491597967852
        a6 = -0.06688050055567513
        a7 = 3.673698118236408
        a8 = 1.490835962046328
        a9 = 0.0328655854088019
        b = 0.3650935010127353

        return 1 / (
            ((a1 + a2 * d + a3 * w**2) * (a4 + a5 * s + a6 * s**2))
            / (a7 + a8 * w + a9 * d**2)
            + b
        )


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
    signal = generate_sine(FREQUENCY, length=LENGTH) * 1
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
        return Hysteresis(drive, saturation, width, FS, makeup=True).process_block(
            block
        )

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
        [0.0, 0.25, 0.5, 0.75, 1.0],
    )

    axs[0, 2].set_title("Width")
    analyze_processor(
        axs,
        2,
        lambda block, width: processor(block, width=width),
        [0.0, 0.25, 0.5, 0.75, 0.99],
    )

    plt.show()


def amplitude_generate():
    D = 20
    S = 20
    W = 20

    input_configs = []
    for d in np.linspace(0.2, 20.0, D):
        for s in np.linspace(0, 1, S):
            for w in np.linspace(0, 0.9999, W):
                input_configs.append(
                    {
                        "d": d,
                        "s": s,
                        "w": w,
                    }
                )

    i = 1
    m = D * S * W
    configs = []
    with concurrent.futures.ProcessPoolExecutor() as executor:
        for config in executor.map(set_max_amplitude, input_configs):
            configs.append(config)
            print(f"{i}/{m}")
            i += 1

    with open(AMPLITUDE_DATASET, "w", newline="") as f:
        writer = DictWriter(f, fieldnames=configs[0].keys())
        writer.writeheader()
        writer.writerows(configs)

    print("Done")


def set_max_amplitude(config):
    FREQUENCY = 100.0
    LENGTH = 0.02
    signal = generate_sine(FREQUENCY, length=LENGTH) * 0.7
    config["a"] = np.max(
        Hysteresis(config["d"], config["s"], config["w"], FS).process_block(signal)
    )
    return config


def amplitude_fitting():
    try:
        data_frame = pd.read_csv(AMPLITUDE_DATASET)
    except FileNotFoundError:
        exit("Dataset not found, generate it first")

    d_data = data_frame["d"].values
    s_data = data_frame["s"].values
    w_data = data_frame["w"].values
    a_data = data_frame["a"].values

    functions = (
        func_n_exp,  # rmse=0.03
        func_o_exp,  # rmse=0.03
        func_d_exp,  # rmse=0.07
        func_q_pow3,  # rmse=0.09
        func_j_pow2,  # rmse=0.11
        func_p_exp,  # rmse=0.12
        func_c_pow2,  # rmse=0.13
        func_f_pow2,  # rmse=0.15
        func_h_pow2,  # rmse=0.15
        func_b_simple,  # rmse=0.17
        func_a_linear,  # rmse=0.18
        func_k_exp,  # rmse=0.19
        func_i_pow2,  # rmse=0.19
        func_g_pow2,  # rmse=0.36
        func_e_pow2,  # rmse=0.37
        func_m_exp,  # rmse=0.41
        func_l_exp,  # rmse=0.44
    )

    input_configs = [(func, d_data, s_data, w_data, a_data) for func in functions]

    print("Testing fitting functions:")
    configs = []
    with concurrent.futures.ProcessPoolExecutor() as executor:
        for i, config in enumerate(
            executor.map(measure_fitting_accuracy, input_configs)
        ):
            configs.append(config)
            print("Processed: {}/{}".format(i + 1, len(input_configs)))

    configs = sorted(configs, key=lambda x: x["rmse"])

    print("Approximations ordered by accuracy:")
    for (i, config) in enumerate(configs):
        print(
            "{}. {}(rmse={}, rs={})".format(
                i + 1, config["func"], config["rmse"], config["rs"]
            )
        )

    while True:
        selected = input("Show parameters (empty to exit): ")
        if selected == "":
            return
        try:
            parameters = configs[int(selected) - 1]["parameters"]
        except:
            print("Invalid index")
            continue
        print_parameters(parameters)


def measure_fitting_accuracy(config):
    f = config[0]
    d_data = config[1]
    s_data = config[2]
    w_data = config[3]
    a_data = config[4]

    try:
        fitted_parameters, _ = curve_fit(
            f, [d_data, s_data, w_data], a_data, maxfev=60000
        )
    except RuntimeError:
        print("Unable to fit")
        return {
            "func": f.__name__,
            "rmse": 1.0,
            "rs": 0.0,
            "parameters": [],
        }

    model_predictions = f([d_data, s_data, w_data, a_data], *fitted_parameters)
    abs_errors = model_predictions - a_data
    squared_errors = np.square(abs_errors)
    mean_squared_errors = np.mean(squared_errors)
    root_mean_squared_errors = np.sqrt(mean_squared_errors)
    r_squared = 1.0 - (np.var(abs_errors) / np.var(a_data))
    return {
        "func": f.__name__,
        "rmse": root_mean_squared_errors,
        "rs": r_squared,
        "parameters": fitted_parameters,
    }


def func_a_linear(data, a1, a2, a3, b):
    d, s, w = data[0], data[1], data[2]
    return a1 * d + a2 * s + a3 * w + b


def func_b_simple(data, a1, a2, a3, a4, a5, a6, b):
    d, s, w = data[0], data[1], data[2]
    return a1 * d + a2 * s + a3 * w + a4 * d * s + a5 * d * w + a6 * s * w + b


def func_c_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return (
        a1 * d
        + a2 * s
        + a3 * w
        + a4 * d * s
        + a5 * d * w
        + a6 * s * w
        + a7 * d**2
        + a8 * s**2
        + a9 * w**2
        + b
    )


def func_d_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    d, s, w = data[0], data[1], data[2]
    return (
        a1 * d
        + a2 * s
        + a3 * w
        + a4 * d * s
        + a5 * d * w
        + a6 * s * w
        + a7 * d**a8
        + a9 * s**a10
        + a11 * w**a12
        + b
    )


def func_e_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return (a1 + a2 * d + a3 * d**2) / (
        (a4 + a5 * s + a6 * s**2) * (a7 + a8 * w + a9 * w**2)
    ) + b


def func_f_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return (a1 + a2 * d + a3 * s**2) / (
        (a4 + a5 * s + a6 * d**2) * (a7 + a8 * w + a9 * w**2)
    ) + b


def func_g_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return (a1 + a2 * d + a3 * w**2) / (
        (a4 + a5 * s + a6 * d**2) * (a7 + a8 * w + a9 * s**2)
    ) + b


def func_h_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return ((a1 + a2 * d + a3 * d**2) * (a4 + a5 * s + a6 * s**2)) / (
        a7 + a8 * w + a9 * w**2
    ) + b


def func_i_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return ((a1 + a2 * d + a3 * s**2) * (a4 + a5 * s + a6 * d**2)) / (
        a7 + a8 * w + a9 * w**2
    ) + b


def func_j_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return ((a1 + a2 * d + a3 * w**2) * (a4 + a5 * s + a6 * s**2)) / (
        a7 + a8 * w + a9 * d**2
    ) + b


def func_k_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return (a1 + a2 * d**a3) / ((a4 + a5 * s**a6) * (a7 + a8 * w**a9)) + b


def func_l_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return (a1 + a2 * s**a3) / ((a4 + a5 * d**a6) * (a7 + a8 * w**a9)) + b


def func_m_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return (a1 + a2 * w**a3) / ((a4 + a5 * d**a6) * (a7 + a8 * s**a9)) + b


def func_n_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return ((a1 + a2 * d**a3) * (a4 + a5 * s**a6)) / (a7 + a8 * w**a9) + b


def func_o_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return ((a1 + a2 * s**a3) * (a4 + a5 * d**a6)) / (a7 + a8 * w**a9) + b


def func_p_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, b):
    d, s, w = data[0], data[1], data[2]
    return ((a1 + a2 * w**a3) * (a4 + a5 * s**a6)) / (a7 + a8 * d**a9) + b


def func_q_pow3(
    data,
    a1,
    a2,
    a3,
    a4,
    a5,
    a6,
    a7,
    a8,
    a9,
    a10,
    a11,
    a12,
    a13,
    a14,
    a15,
    a16,
    a17,
    a18,
    a19,
    b,
):
    d, s, w = data[0], data[1], data[2]
    return (
        a1 * d
        + a2 * s
        + a3 * w
        + a4 * d**2
        + a5 * s**2
        + a6 * w**2
        + a7 * d**3
        + a8 * s**3
        + a9 * w**3
        + a10 * d * s
        + a11 * d * w
        + a12 * s * w
        + a13 * d**2 * s
        + a14 * d * s**2
        + a15 * d**2 * w
        + a16 * d * w**2
        + a17 * s**2 * w
        + a18 * s * w**2
        + a19 * d * s * w
        + b
    )


def print_parameters(parameters):
    for (i, a) in enumerate(parameters[: len(parameters) - 1]):
        print("a{} = {}".format(i + 1, a))
    print("b = {}".format(parameters[-1]))


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

    subparsers.add_parser(
        "amplitude_fitting", help="Find the best fitting approximation for amplitude"
    )
    args = parser.parse_args()

    if args.subparser == "response":
        response()
    elif args.subparser == "amplitude_generate":
        amplitude_generate()
    elif args.subparser == "amplitude_fitting":
        amplitude_fitting()
