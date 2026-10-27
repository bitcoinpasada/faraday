//! Free memory on macOS (`docs/VAULT.md` §3.1): `vm_stat`'s free,
//! inactive and speculative pages times the page size it prints, on
//! Apple Silicon's 16 KiB pages and Intel's 4 KiB; anything else read is
//! not a number, and a vault then opens only at Light's memory.

use faraday_storage::vm_stat_available_mib;

/// `vm_stat` as an Apple Silicon Mac prints it.
const APPLE_SILICON: &str = "Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                                3221.
Pages active:                            321571.
Pages inactive:                          316745.
Pages speculative:                         2337.
Pages throttled:                              0.
Pages wired down:                        154829.
Pages purgeable:                           4612.
\"Translation faults\":                 952934578.
Pages copy-on-write:                   27153590.
Pages zero filled:                    459340713.
Pages reactivated:                      9327541.
Pages purged:                           2245614.
File-backed pages:                       201837.
Anonymous pages:                         439816.
Pages stored in compressor:              601349.
Pages occupied by compressor:            222048.
Decompressions:                        13012474.
Compressions:                          18766309.
Pageins:                               41255237.
Pageouts:                                194306.
Swapins:                                      0.
Swapouts:                                     0.
";

/// `vm_stat` as an Intel Mac prints it.
const INTEL: &str = "Mach Virtual Memory Statistics: (page size of 4096 bytes)
Pages free:                               45678.
Pages active:                            987654.
Pages inactive:                          876543.
Pages speculative:                        12345.
Pages throttled:                              0.
Pages wired down:                        456789.
Pages purgeable:                          23456.
\"Translation faults\":                 123456789.
Pages copy-on-write:                    3456789.
";

#[test]
fn apple_silicon_16k_pages_give_free_inactive_and_speculative() {
    // (3221 + 316745 + 2337) × 16384 bytes = 5035.98 MiB.
    assert_eq!(vm_stat_available_mib(APPLE_SILICON), Some(5035));
}

#[test]
fn intel_4k_pages_give_free_inactive_and_speculative() {
    // (45678 + 876543 + 12345) × 4096 bytes = 3650.65 MiB.
    assert_eq!(vm_stat_available_mib(INTEL), Some(3650));
}

#[test]
fn output_missing_a_count_or_the_page_size_reads_as_unknown() {
    let no_speculative: String = INTEL
        .lines()
        .filter(|l| !l.starts_with("Pages speculative"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(vm_stat_available_mib(&no_speculative), None);
    let no_size = INTEL.replacen("(page size of 4096 bytes)", "", 1);
    assert_eq!(vm_stat_available_mib(&no_size), None);
    assert_eq!(vm_stat_available_mib(""), None);
    let garbled = INTEL.replacen("45678.", "lots.", 1);
    assert_eq!(vm_stat_available_mib(&garbled), None);
}
