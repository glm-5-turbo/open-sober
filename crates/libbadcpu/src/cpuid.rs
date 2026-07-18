// SPDX-License-Identifier: MIT
//
// CPU feature detection via CPUID instruction.
// Determines available x86-64 features at runtime.

use core::arch::x86_64::__cpuid;
use core::arch::x86_64::__cpuid_count;

/// CPU feature set detected at runtime.
#[derive(Debug, Clone)]
pub struct CpuFeatures {
    pub vendor: CpuVendor,
    pub has_sse3: bool,
    pub has_ssse3: bool,
    pub has_sse41: bool,
    pub has_sse42: bool,
    pub has_popcnt: bool,
    pub has_avx: bool,
    pub has_fma: bool,
    pub has_avx2: bool,
    pub has_bmi1: bool,
    pub has_bmi2: bool,
    pub brand_string: [u8; 48],
}

impl Default for CpuFeatures {
    fn default() -> Self {
        Self {
            vendor: CpuVendor::default(),
            has_sse3: false,
            has_ssse3: false,
            has_sse41: false,
            has_sse42: false,
            has_popcnt: false,
            has_avx: false,
            has_fma: false,
            has_avx2: false,
            has_bmi1: false,
            has_bmi2: false,
            brand_string: [0u8; 48],
        }
    }
}

/// CPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CpuVendor {
    #[default]
    Unknown,
    Intel,
    AMD,
    Hygon,
}

/// Detect CPU features using CPUID.
pub fn detect_features() -> CpuFeatures {
    let mut features = CpuFeatures::default();

    // Leaf 0: vendor string and max standard leaf
    let result = unsafe { __cpuid(0u32) };
    let max_std = result.eax as u32;

    let vendor_bytes: [u8; 12] = [
        result.ebx as u8, (result.ebx >> 8) as u8, (result.ebx >> 16) as u8, (result.ebx >> 24) as u8,
        result.edx as u8, (result.edx >> 8) as u8, (result.edx >> 16) as u8, (result.edx >> 24) as u8,
        result.ecx as u8, (result.ecx >> 8) as u8, (result.ecx >> 16) as u8, (result.ecx >> 24) as u8,
    ];

    let vendor_str = core::str::from_utf8(&vendor_bytes).unwrap_or("");
    features.vendor = match vendor_str {
        "GenuineIntel" => CpuVendor::Intel,
        "AuthenticAMD" => CpuVendor::AMD,
        "HygonGenuine" => CpuVendor::Hygon,
        _ => CpuVendor::Unknown,
    };

    // Leaf 1: feature bits
    if max_std >= 1 {
        let result = unsafe { __cpuid(1u32) };
        let ecx = result.ecx as u32;
        let _edx = result.edx as u32;

        features.has_sse3 = (ecx & (1 << 0)) != 0;
        features.has_ssse3 = (ecx & (1 << 9)) != 0;
        features.has_sse41 = (ecx & (1 << 19)) != 0;
        features.has_sse42 = (ecx & (1 << 20)) != 0;
        features.has_popcnt = (ecx & (1 << 23)) != 0;
        features.has_avx = (ecx & (1 << 28)) != 0;
        features.has_fma = (ecx & (1 << 12)) != 0;
    }

    // Leaf 7 (subleaf 0): extended features
    if max_std >= 7 {
        let result = unsafe { __cpuid_count(7u32, 0u32) };
        let ebx = result.ebx as u32;

        features.has_avx2 = (ebx & (1 << 5)) != 0;
        features.has_bmi1 = (ebx & (1 << 3)) != 0;
        features.has_bmi2 = (ebx & (1 << 8)) != 0;
    }

    // Extended leaves: brand string
    let ext_result = unsafe { __cpuid(0x8000_0000u32) };
    let max_ext = ext_result.eax as u32;

    if max_ext >= 0x8000_0004 {
        for i in 0usize..3 {
            let result = unsafe { __cpuid(0x8000_0002 + i as u32) };
            let offset = i * 16;
            features.brand_string[offset..offset + 4].copy_from_slice(&result.eax.to_le_bytes());
            features.brand_string[offset + 4..offset + 8].copy_from_slice(&result.ebx.to_le_bytes());
            features.brand_string[offset + 8..offset + 12].copy_from_slice(&result.ecx.to_le_bytes());
            features.brand_string[offset + 12..offset + 16].copy_from_slice(&result.edx.to_le_bytes());
        }
    }

    features
}

/// Print CPU information to stderr.
pub fn print_info(features: &CpuFeatures) {
    let vendor_str = match features.vendor {
        CpuVendor::Intel => "GenuineIntel",
        CpuVendor::AMD => "AuthenticAMD",
        CpuVendor::Hygon => "HygonGenuine",
        CpuVendor::Unknown => "Unknown",
    };

    eprintln!("CPU Vendor: {}", vendor_str);

    let brand = core::str::from_utf8(&features.brand_string).unwrap_or("");
    let brand_trimmed = brand.trim_end_matches('\0').trim();
    if !brand_trimmed.is_empty() {
        eprintln!("CPU Brand: {}", brand_trimmed);
    }

    eprintln!("CPU Feature Support:");
    eprintln!("  SSE3: {}", if features.has_sse3 { "yes" } else { "no" });
    eprintln!("  SSSE3: {}", if features.has_ssse3 { "yes" } else { "no" });
    eprintln!("  SSE4.1: {}", if features.has_sse41 { "yes" } else { "no" });
    eprintln!("  SSE4.2: {}", if features.has_sse42 { "yes" } else { "no" });
    eprintln!("  POPCNT: {}", if features.has_popcnt { "yes" } else { "no" });
    eprintln!("  AVX: {}", if features.has_avx { "yes" } else { "no" });
    eprintln!("  AVX2: {}", if features.has_avx2 { "yes" } else { "no" });
    eprintln!("  BMI1: {}", if features.has_bmi1 { "yes" } else { "no" });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_features() {
        let features = detect_features();
        assert!(features.has_sse3, "Expected SSE3 support");
        assert!(
            matches!(features.vendor, CpuVendor::Intel | CpuVendor::AMD),
            "Expected Intel or AMD CPU"
        );
    }
}