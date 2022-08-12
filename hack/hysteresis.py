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
        x1 = max(drive, 0.1)
        x2 = saturation
        x3 = width
        x4 = 1.0

        # func_u_13_exp
        a1 = 0.8520741612250788
        a2 = -0.8525619069449913
        a3 = 1.9902211821492115
        a4 = -1.4660475978317764
        a5 = -4.033266657270668
        a6 = 1.2645654729014488
        a7 = 0.0018519329228452867
        a8 = 0.0015962762890290784
        a9 = 0.00011242579498388368
        a10 = 1.0609808177687021
        a11 = 1.9030561556321686
        a12 = -1.186218831819727
        b = 0.02887903913851876

        return 1 / (((a1 + a2 * x4**a9) * (a3 + a4 * x2**a10) * (a5 + a6 * x3**a11)) / (a7 + a8 * x1**a12) + b)

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
    signal = generate_sine(FREQUENCY, length=LENGTH) * 0.2
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
        return Hysteresis(drive, saturation, width, FS, makeup=True).process_block(block)

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

    with open(AMPLITUDE_DATASET, "w", newline="") as f:
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


def amplitude_fitting():
    try:
        data_frame = pd.read_csv(AMPLITUDE_DATASET)
    except FileNotFoundError:
        exit("Dataset not found, generate it first")

    d_data = data_frame["d"].values
    s_data = data_frame["s"].values
    w_data = data_frame["w"].values
    i_data = data_frame["i"].values
    a_data = data_frame["a"].values


    functions = (
        func_u_13_exp, # rmse=0.08
        # func_u_11_exp_opt1, # rmse=0.18
        # func_u_12_exp_opt2, # rmse=0.39

        # func_e_31_pow3, # rmse=0.09
        # func_e_28_pow3_opt1, # rmse=0.12
        # func_e_23_pow3_opt2, # rmse=0.13

        # func_m_13_exp, # rmse=0.09
        # func_m_12_exp_opt1, # rmse=0.09
        # func_m_5_exp_opt2, # rmse=0.40

        # func_q_13_exp, # rmse=0.09

        # func_r_13_exp, # rmse=0.11

        # func_t_13_exp, # rmse=0.11

        # func_c_15_pow2, # rmse=0.13
        # func_c_13_pow2_opt1, # rmse=0.17

        # func_l_13_exp, # rmse=0.13

        # func_gg_13_pow2, # rmse=0.14
        # func_hh_13_pow2, # rmse=0.14
        # func_w_13_pow2, # rmse=0.14
        # func_x_13_pow2, # rmse=0.14
        # func_ii_13_pow2, # rmse=0.14
        # func_y_13_pow2, # rmse=0.14
        # func_d_13_pow3, # rmse=0.14
        # func_a_5, # rmse=0.20
        # func_b_9, # rmse=0.16
        # func_f_9, # rmse=0.19
        # func_g_9, # rmse=0.18
        # func_h_13, # rmse=0.40
        # func_i_13, # rmse=0.36
        # func_j_13, # Unable to fit
        # func_k_13, # Unable to fit
        # func_n_13, # Unable to fit
        # func_o_13, # rmse=0.46
        # func_p_13, # rmse=0.42
        # func_s_13, # rmse=0.40
        # func_z_13, # rmse=0.32
        # func_aa_13, # Unable to fit
        # func_bb_13, # rmse=0.38
        # func_cc_13, # rmse=0.41
        # func_dd_13, # Unable to fit
        # func_ee_13, # rmse=0.43
        # func_ff_13, # rmse=0.43
        # func_jj_13, # Unable to fit
    )

    input_configs = [
        (func, d_data, s_data, w_data, i_data, a_data) for func in functions
    ]

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
    i_data = config[4]
    a_data = config[5]

    try:
        fitted_parameters, _ = curve_fit(
            f, [d_data, s_data, w_data, i_data], a_data, maxfev=60000
        )
    except RuntimeError:
        print("Unable to fit")
        return {
            "func": f.__name__,
            "rmse": 1.0,
            "rs": 0.0,
            "parameters": [],
        }

    model_predictions = f([d_data, s_data, w_data, i_data, a_data], *fitted_parameters)
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


def func_a_5(data, a1, a2, a3, a4, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return a1 * x1 + a2 * x2 + a3 * x3 + a4 * x4 + b


def func_b_9(data, a1, a2, a3, a4, a5, a6, a7, a8, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (
        a1 * x1
        + a2 * x2
        + a3 * x3
        + a4 * x4
        + a5 * x1**2
        + a6 * x2**2
        + a7 * x3**2
        + a8 * x4**2
        + b
    )


def func_c_15_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (
        a1 * x1
        + a2 * x2
        + a3 * x3
        + a4 * x4
        + a5 * x1**2
        + a6 * x2**2
        + a7 * x3**2
        + a8 * x4**2
        + a9 * x1 * x2
        + a10 * x1 * x3
        + a11 * x1 * x4
        + a12 * x2 * x3
        + a13 * x2 * x4
        + a14 * x3 * x4
        + b
    )


# Removed 10^-3, a5, a10
def func_c_13_pow2_opt1(data, a1, a2, a3, a4, a6, a7, a8, a9, a11, a12, a13, a14, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (
        a1 * x1
        + a2 * x2
        + a3 * x3
        + a4 * x4
        + a6 * x2**2
        + a7 * x3**2
        + a8 * x4**2
        + a9 * x1 * x2
        + a11 * x1 * x4
        + a12 * x2 * x3
        + a13 * x2 * x4
        + a14 * x3 * x4
        + b
    )


def func_d_13_pow3(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (
        a1 * x1
        + a2 * x2
        + a3 * x3
        + a4 * x4
        + a5 * x1**2
        + a6 * x2**2
        + a7 * x3**2
        + a8 * x4**2
        + a9 * x1**3
        + a10 * x2**3
        + a11 * x3**3
        + a12 * x4**3
        + b
    )


def func_e_31_pow3(
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
    a20,
    a21,
    a22,
    a23,
    a24,
    a25,
    a26,
    a27,
    a28,
    a29,
    a30,
    b,
):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (
        a1 * x1
        + a2 * x2
        + a3 * x3
        + a4 * x4
        + a5 * x1**2
        + a6 * x2**2
        + a7 * x3**2
        + a8 * x4**2
        + a9 * x1**3
        + a10 * x2**3
        + a11 * x3**3
        + a12 * x4**3
        + a13 * x1 * x2
        + a14 * x1 * x3
        + a15 * x1 * x4
        + a16 * x2 * x3
        + a17 * x2 * x4
        + a18 * x3 * x4
        + a19 * x1**2 * x2
        + a20 * x1**2 * x3
        + a21 * x1**2 * x4
        + a22 * x2**2 * x3
        + a23 * x2**2 * x4
        + a24 * x3**2 * x4
        + a25 * x1 * x2**2
        + a26 * x1 * x3**2
        + a27 * x1 * x4**2
        + a28 * x2 * x3**2
        + a29 * x2 * x4**2
        + a30 * x3 * x4**2
        + b
    )


# removed 10^-4 elements, a9, a20, a21,
def func_e_28_pow3_opt1(
    data,
    a1,
    a2,
    a3,
    a4,
    a5,
    a6,
    a7,
    a8,
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
    a22,
    a23,
    a24,
    a25,
    a26,
    a27,
    a28,
    a29,
    a30,
    b,
):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (
        a1 * x1
        + a2 * x2
        + a3 * x3
        + a4 * x4
        + a5 * x1**2
        + a6 * x2**2
        + a7 * x3**2
        + a8 * x4**2
        + a10 * x2**3
        + a11 * x3**3
        + a12 * x4**3
        + a13 * x1 * x2
        + a14 * x1 * x3
        + a15 * x1 * x4
        + a16 * x2 * x3
        + a17 * x2 * x4
        + a18 * x3 * x4
        + a19 * x1**2 * x2
        + a22 * x2**2 * x3
        + a23 * x2**2 * x4
        + a24 * x3**2 * x4
        + a25 * x1 * x2**2
        + a26 * x1 * x3**2
        + a27 * x1 * x4**2
        + a28 * x2 * x3**2
        + a29 * x2 * x4**2
        + a30 * x3 * x4**2
        + b
    )


# removed 10^-3 elements, a9, a20, a21, a19, a22, a25, a26, b
def func_e_23_pow3_opt2(
    data,
    a1,
    a2,
    a3,
    a4,
    a5,
    a6,
    a7,
    a8,
    a10,
    a11,
    a12,
    a13,
    a14,
    a15,
    a16,
    a17,
    a18,
    a23,
    a24,
    a27,
    a28,
    a29,
    a30,
):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (
        a1 * x1
        + a2 * x2
        + a3 * x3
        + a4 * x4
        + a5 * x1**2
        + a6 * x2**2
        + a7 * x3**2
        + a8 * x4**2
        + a10 * x2**3
        + a11 * x3**3
        + a12 * x4**3
        + a13 * x1 * x2
        + a14 * x1 * x3
        + a15 * x1 * x4
        + a16 * x2 * x3
        + a17 * x2 * x4
        + a18 * x3 * x4
        + a23 * x2**2 * x4
        + a24 * x3**2 * x4
        + a27 * x1 * x4**2
        + a28 * x2 * x3**2
        + a29 * x2 * x4**2
        + a30 * x3 * x4**2
    )


def func_f_9(data, a1, a2, a3, a4, a5, a6, a7, a8, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1) * (a3 + a4 * x2)) / ((a5 + a6 * x3) * (a7 + a8 * x4)) + b


def func_g_9(data, a1, a2, a3, a4, a5, a6, a7, a8, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1) * (a3 + a4 * x2)) * ((a5 + a6 * x3) * (a7 + a8 * x4)) + b


def func_h_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1**a9) * (a3 + a4 * x2**a10)) / (
        (a5 + a6 * x3**a11) * (a7 + a8 * x4**a12)
    ) + b


def func_i_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1**a9) * (a3 + a4 * x3**a10)) / (
        (a5 + a6 * x2**a11) * (a7 + a8 * x4**a12)
    ) + b


def func_j_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1**a9) * (a3 + a4 * x4**a10)) / (
        (a5 + a6 * x2**a11) * (a7 + a8 * x3**a12)
    ) + b


def func_k_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x2**a9) * (a3 + a4 * x3**a10)) / (
        (a5 + a6 * x1**a11) * (a7 + a8 * x4**a12)
    ) + b


def func_l_13_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x2**a9) * (a3 + a4 * x4**a10)) / (
        (a5 + a6 * x1**a11) * (a7 + a8 * x3**a12)
    ) + b


def func_m_13_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x3**a9) * (a3 + a4 * x4**a10)) / (
        (a5 + a6 * x1**a11) * (a7 + a8 * x2**a12)
    ) + b


# Removing 10^-4, a10
def func_m_12_exp_opt1(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x3**a9) * (a3 + a4 * x4)) / (
        (a5 + a6 * x1**a11) * (a7 + a8 * x2**a12)
    ) + b


# Removing 10^-2, a10, a5, a6, a7, a8, a11, a12, b
def func_m_5_exp_opt2(data, a1, a2, a3, a4, a9, a12):
    x3, x4 = data[2], data[3]
    return (a1 + a2 * x3**a9) * (a3 + a4 * x4)


def func_n_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (a1 + a2 * x1**a9) / ((a3 + a4 * x2**a10) *
        (a5 + a6 * x3**a11) * (a7 + a8 * x4**a12)
    ) + b


def func_o_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (a1 + a2 * x2**a9) / ((a3 + a4 * x1**a10) *
        (a5 + a6 * x3**a11) * (a7 + a8 * x4**a12)
    ) + b


def func_p_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (a1 + a2 * x3**a9) / ((a3 + a4 * x1**a10) *
        (a5 + a6 * x2**a11) * (a7 + a8 * x4**a12)
    ) + b


def func_q_13_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (a1 + a2 * x4**a9) / ((a3 + a4 * x1**a10) *
        (a5 + a6 * x2**a11) * (a7 + a8 * x3**a12)
    ) + b


def func_r_13_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1**a9) * (a3 + a4 * x2**a10) * (a5 + a6 * x3**a11)) / (a7 + a8 * x4**a12) + b


def func_s_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1**a9) * (a3 + a4 * x2**a10) * (a5 + a6 * x4**a11)) / (a7 + a8 * x3**a12) + b


def func_t_13_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1**a9) * (a3 + a4 * x4**a10) * (a5 + a6 * x3**a11)) / (a7 + a8 * x2**a12) + b


def func_u_13_exp(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x4**a9) * (a3 + a4 * x2**a10) * (a5 + a6 * x3**a11)) / (a7 + a8 * x1**a12) + b


# a9 replaced with 0, x4 removed
def func_u_11_exp_opt1(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, b):
    x1, x2, x3 = data[0], data[1], data[2]
    return (a1 * (a2 + a3 * x2**a8) * (a4 + a5 * x3**a9)) / (a6 + a7 * x1**a10) + b


# a10 removed
def func_u_12_exp_opt2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x4**a9) * (a3 + a4 * x2) * (a5 + a6 * x3**a10)) / (a7 + a8 * x1**a11) + b


def func_w_13_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1 + a9 * x1 **2) * (a3 + a4 * x2 + a10 * x2 **2)) / (
        (a5 + a6 * x3 + a11 * x3**2) * (a7 + a8 * x4 + a12 * x4**2)
    ) + b


def func_x_13_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1 + a9 * x1 ** 2) * (a3 + a4 * x3 + a10 * x3 **2)) / (
        (a5 + a6 * x2 + a11 * x2 ** 2) * (a7 + a8 * x4 + a12 ** x4 ** 2)
    ) + b


def func_y_13_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1 + a9 * x1 **2) * (a3 + a4 * x4 + a10 * x4 **2)) / (
        (a5 + a6 * x2 + a11 * x2 ** 2) * (a7 + a8 * x3 + a12 * x3 ** 2)
    ) + b


def func_z_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x2 + a9 * x2 ** 2) * (a3 + a4 * x3 + a10) * x3 ** 2) / (
        (a5 + a6 * x1 + a11 * x1 **2) * (a7 + a8 * x4 + a12 * x4 ** 2)
    ) + b


def func_aa_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x2 + a9 * x2 ** 2) * (a3 + a4 * x4 + a10 * x4 ** 2)) / (
        (a5 + a6 * x1 + a11 * x1 ** 2) * (a7 + a8 * x3 + a12 * x3 ** 2)
    ) + b


def func_bb_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x3 + a9 * x3 ** 2) * (a3 + a4 * x4 + a10 * x4 ** 2)) / (
        (a5 + a6 * x1 + a11 * x1 ** 2) * (a7 + a8 * x2 + a12 * x2 ** 2)
    ) + b


def func_cc_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (a1 + a2 * x1 + a9 * x1 ** 2) / ((a3 + a4 * x2 + a10 * x2 ** 2) *
        (a5 + a6 * x3 + a11 * x3 ** 2) * (a7 + a8 * x4 + a12 * x4 ** 2)
    ) + b


def func_dd_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (a1 + a2 * x2 + a9 * x2 ** 2) / ((a3 + a4 * x1 + a10 * x1 ** 2) *
        (a5 + a6 * x3 + a11 * x3 ** 2) * (a7 + a8 * x4 + a12 * x4 ** 2)
    ) + b


def func_ee_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (a1 + a2 * x3 + a9 * x3 ** 2) / ((a3 + a4 * x1 + a10 * x1 ** 2) *
        (a5 + a6 * x2 + a11 * x2 ** 2) * (a7 + a8 * x4 + a12 * x4 ** 2)
    ) + b


def func_ff_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return (a1 + a2 * x4 + a9 * x4 ** 2) / ((a3 + a4 * x1 + a10 * x1 ** 2) *
        (a5 + a6 * x2 + a11 * x2 ** 2) * (a7 + a8 * x3 + a12 * x3 ** 2)
    ) + b


def func_gg_13_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1 + a9 * x1 ** 2) * (a3 + a4 * x2 + a10 * x2 ** 2) * (a5 + a6 * x3 + a11 * x3 ** 2)) / (a7 + a8 * x4 + a12 * x4 ** 2) + b


def func_hh_13_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1 + a9 * x1 ** 2) * (a3 + a4 * x2 + a10 * x2 ** 2) * (a5 + a6 * x4 + a11 * x4 ** 2)) / (a7 + a8 * x3 + a12 * x3 ** 2) + b


def func_ii_13_pow2(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x1 + a9 * x1 ** 2) * (a3 + a4 * x4 + a10 * x4 ** 2) * (a5 + a6 * x3 + a11 * x3 ** 2)) / (a7 + a8 * x2 + a12 * x2 ** 2) + b


def func_jj_13(data, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, b):
    x1, x2, x3, x4 = data[0], data[1], data[2], data[3]
    return ((a1 + a2 * x4 + a9 * x4 ** 2) * (a3 + a4 * x2 + a10* x2 **2) * (a5 + a6 * x3 + a11 * x3 ** 2)) / (a7 + a8 * x1 + a12 * x1**2) + b


def print_parameters(parameters):
    for (i, a) in enumerate(parameters[: len(parameters) - 1]):
        print("a{} = {}".format(i+1, a))
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
