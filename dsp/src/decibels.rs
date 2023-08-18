#[allow(unused_imports)]
use micromath::F32Ext as _;

// Generated using Python:
//
// ```
// import math
//
// for i in range(256):
//     min = -40
//     max = 0
//     diff = max - min
//     db = min + (i / 255) * diff
//     print("{},".format(pow(10, db / 20)))
// ```
const DB_TO_LINEAR: [f32; 256] = [
    0.01,
    0.010_182_235,
    0.010_367_792,
    0.010_556_729_5,
    0.010_749_111,
    0.010_944_998,
    0.011_144_455,
    0.011_347_546,
    0.011_554_339,
    0.011_764_9,
    0.011_979_298,
    0.012_197_603_5,
    0.012_419_887,
    0.012_646_222,
    0.012_876_68,
    0.013_111_339,
    0.013_350_274,
    0.013_593_564,
    0.013_841_287,
    0.014_093_524,
    0.014_350_358,
    0.014_611_873,
    0.014_878_153,
    0.015_149_286,
    0.015_425_36,
    0.015_706_465,
    0.015_992_692,
    0.016_284_136,
    0.016_580_89,
    0.016_883_053,
    0.017_190_723,
    0.017_503_997,
    0.017_822_983,
    0.018_147_781,
    0.018_478_498,
    0.018_815_242,
    0.019_158_123,
    0.019_507_252,
    0.019_862_743,
    0.020_224_713,
    0.020_593_278,
    0.020_968_56,
    0.021_350_682,
    0.021_739_768,
    0.022_135_943,
    0.022_539_34,
    0.022_950_087,
    0.023_368_318,
    0.023_794_172,
    0.024_227_785,
    0.024_669_303,
    0.025_118_865,
    0.025_576_62,
    0.026_042_717,
    0.026_517_307,
    0.027_000_546,
    0.027_492_592,
    0.027_993_605,
    0.028_503_748,
    0.029_023_187,
    0.029_552_093,
    0.030_090_636,
    0.030_638_995,
    0.031_197_347,
    0.031_765_87,
    0.032_344_76,
    0.032_934_196,
    0.033_534_374,
    0.034_145_49,
    0.034_767_74,
    0.035_401_333,
    0.036_046_47,
    0.036_703_367,
    0.037_372_23,
    0.038_053_285,
    0.038_746_75,
    0.039_452_855,
    0.040_171_824,
    0.040_903_9,
    0.041_649_31,
    0.042_408_31,
    0.043_181_14,
    0.043_968_055,
    0.044_769_31,
    0.045_585_167,
    0.046_415_888,
    0.047_261_752,
    0.048_123_028,
    0.049,
    0.049_892_955,
    0.050_802_18,
    0.051_727_977,
    0.052_670_643,
    0.053_630_49,
    0.054_607_827,
    0.055_602_975,
    0.056_616_26,
    0.057_648_01,
    0.058_698_56,
    0.059_768_256,
    0.060_857_445,
    0.061_966_486,
    0.063_095_73,
    0.064_245_56,
    0.065_416_34,
    0.066_608_466,
    0.067_822_31,
    0.069_058_27,
    0.070_316_754,
    0.071_598_18,
    0.072_902_95,
    0.074_231_5,
    0.075_584_26,
    0.076_961_674,
    0.078_364_186,
    0.079_792_26,
    0.081_246_36,
    0.082_726_955,
    0.084_234_536,
    0.085_769_586,
    0.087_332_614,
    0.088_924_125,
    0.090_544_64,
    0.092_194_684,
    0.093_874_8,
    0.095_585_53,
    0.097_327_44,
    0.099_101_09,
    0.100_907_065,
    0.102_745_95,
    0.104_618_34,
    0.106_524_86,
    0.108_466_126,
    0.110_442_76,
    0.112_455_42,
    0.114_504_755,
    0.116_591_44,
    0.118_716_15,
    0.120_879_58,
    0.123_082_44,
    0.125_325_43,
    0.127_609_31,
    0.129_934_8,
    0.132_302_67,
    0.134_713_7,
    0.137_168_66,
    0.139_668_36,
    0.142_213_61,
    0.144_805_25,
    0.147_444_11,
    0.150_131_08,
    0.152_866_99,
    0.155_652_78,
    0.158_489_32,
    0.161_377_56,
    0.164_318_43,
    0.167_312_89,
    0.170_361_94,
    0.173_466_53,
    0.176_627_71,
    0.179_846_48,
    0.183_123_93,
    0.186_461_09,
    0.189_859_08,
    0.193_319,
    0.196_841_94,
    0.200_429_1,
    0.204_081_64,
    0.207_800_73,
    0.211_587_6,
    0.215_443_46,
    0.219_369_62,
    0.223_367_3,
    0.227_437_85,
    0.231_582_58,
    0.235_802_83,
    0.240_1,
    0.244_475_47,
    0.248_930_68,
    0.253_467_08,
    0.258_086_14,
    0.262_789_4,
    0.267_578_36,
    0.272_454_6,
    0.277_419_7,
    0.282_475_23,
    0.287_622_96,
    0.292_864_44,
    0.298_201_47,
    0.303_635_78,
    0.309_169_08,
    0.314_803_24,
    0.320_540_1,
    0.326_381_47,
    0.332_329_3,
    0.338_385_52,
    0.344_552_1,
    0.350_831_06,
    0.357_224_46,
    0.363_734_33,
    0.370_362_88,
    0.377_112_2,
    0.383_984_54,
    0.390_982_1,
    0.398_107_17,
    0.405_362_1,
    0.412_749_23,
    0.420_270_98,
    0.427_929_82,
    0.435_728_22,
    0.443_668_72,
    0.451_753_94,
    0.459_986_5,
    0.468_369_1,
    0.476_904_45,
    0.485_595_35,
    0.494_444_6,
    0.503_455_16,
    0.512_629_87,
    0.521_971_8,
    0.531_484,
    0.541_169_5,
    0.551_031_53,
    0.561_073_3,
    0.571_298_06,
    0.581_709_15,
    0.592_309_95,
    0.603_103_94,
    0.614_094_6,
    0.625_285_6,
    0.636_680_54,
    0.648_283_1,
    0.660_097_1,
    0.672_126_4,
    0.684_375,
    0.696_846_7,
    0.709_545_73,
    0.722_476_2,
    0.735_642_25,
    0.749_048_3,
    0.762_698_6,
    0.776_597_7,
    0.790_75,
    0.805_160_3,
    0.819_833_16,
    0.834_773_4,
    0.849_985_96,
    0.865_475_8,
    0.881_247_76,
    0.897_307_3,
    0.913_659_4,
    0.930_309_5,
    0.947_263,
    0.964_525_5,
    0.982_102_63,
    1.0,
];

pub fn db_to_linear(db: f32) -> f32 {
    const MIN_DB: f32 = -40.0;
    const MAX_DB: f32 = 1.0;

    if db < MIN_DB {
        return 0.01;
    }

    let phase = (db - MIN_DB) / (MAX_DB - MIN_DB);
    let phase = phase * 256.0;
    let index = phase as usize;
    if index >= 254 {
        return 1.0;
    }

    let a = DB_TO_LINEAR[index];
    let b = DB_TO_LINEAR[index + 1];
    a + (b - a) * phase.fract()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_to_linear_lookup_matches() {
        let min = -40.0;
        let max = 0.0;
        for i in 0..=1024 {
            let db = min + (i as f32 / 1024.0) * (max - min);
            let expected_lin = f32::powf(10.0, db / 20.0);
            assert_relative_eq!(db_to_linear(db), expected_lin, max_relative = 0.1);
        }
    }

    #[test]
    fn db_to_linear_lookup_below_bottom_limit() {
        assert_relative_eq!(db_to_linear(-100.0), 0.01, epsilon = 0.0001);
    }

    #[test]
    fn db_to_linear_lookup_above_bottom_limit() {
        assert_relative_eq!(db_to_linear(-39.99999), 0.01, epsilon = 0.0001);
    }

    #[test]
    fn db_to_linear_lookup_above_top_limit() {
        assert_relative_eq!(db_to_linear(0.999999), 1.0, epsilon = 0.0001);
    }

    #[test]
    fn db_to_linear_lookup_below_top_limit() {
        assert_relative_eq!(db_to_linear(10.0), 1.0, epsilon = 0.0001);
    }
}
