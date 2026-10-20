//! MMC5 (mapper 5) — enough of it to run the Chinese-translated Zelda II.
//!
//! # Why this exists
//!
//! The Chinese ROM (`[20260806_NOKOH]`) is mapper 5 with 256 KiB PRG and
//! 512 KiB CHR. Upstream z2rs only speaks MMC1, so the ROM was rejected at
//! load time (`LoadError::WrongMapper`). Implementing MMC5 is the only way
//! to actually *run* the translated ROM rather than re-deriving its text.
//!
//! # Scope
//!
//! Implemented: PRG banking (all 4 modes), CHR banking (all 4 modes, 1 KiB
//! granularity, with the `$5128-$512B` / `$5130` upper bits), the `$5105`
//! nametable mapping register (read back by the PPU), ExRAM, the scanline
//! IRQ (`$5203`/`$5204`), the hardware multiplier (`$5205`/`$5206`) and
//! PRG-RAM banking (`$5113`).
//!
//! Not implemented: ExRAM *attribute* mode in the PPU renderer and split
//! screen modes. Those only change colours/scroll, not program flow, so the
//! game runs without them; see `PpuBind::sync_from_mapper` for the hook
//! where attribute mode would land.
//!
//! # Register map (NESdev)
//!
//! | Addr | Meaning |
//! |---|---|
//! | `$5100` | PRG mode (bits 1-0) |
//! | `$5101` | CHR mode (bits 1-0) |
//! | `$5102`/`$5103` | PRG-RAM protect |
//! | `$5104` | ExRAM mode |
//! | `$5105` | nametable mapping (2 bits per screen) |
//! | `$5106`/`$5107` | fill-mode tile / attribute |
//! | `$5113` | PRG-RAM bank |
//! | `$5114-$5117` | PRG banks, 8 KiB each |
//! | `$5120-$5127` | CHR banks, 1 KiB each |
//! | `$5128-$512B` | upper 2 bits for two CHR banks each |
//! | `$5130` | upper CHR bits (bits 1-0) |
//! | `$5203`/`$5204` | IRQ target / enable+status |
//! | `$5205`/`$5206` | 8x8 multiplier (write A, write B, read product) |
//! | `$5C00-$5CFF` | ExRAM |

/// 8 KiB: MMC5's PRG banking granularity.
pub const PRG_BANK_LEN: usize = 0x2000;
/// 1 KiB: MMC5's CHR banking granularity.
pub const CHR_BANK_LEN: usize = 0x0400;
/// 4 KiB: what one PPU pattern-table slot holds.
pub const CHR_SLOT_LEN: usize = 0x1000;

/// MMC5 register file + ExRAM.
#[derive(Debug, Clone)]
pub struct Mmc5 {
    /// `$5100`: PRG mode, bits 1-0.
    pub prg_mode: u8,
    /// `$5101`: CHR mode, bits 1-0.
    pub chr_mode: u8,
    /// `$5102`/`$5103`: PRG-RAM protect bits.
    pub prg_ram_protect: u8,
    /// `$5104`: ExRAM mode (0 = plain RAM, 1 = extended attributes, ...).
    pub exram_mode: u8,
    /// `$5105`: nametable mapping, 2 bits per physical screen.
    pub nt_map: u8,
    /// `$5106`: fill-mode tile index.
    pub fill_tile: u8,
    /// `$5107`: fill-mode attribute.
    pub fill_attr: u8,
    /// `$5113`: PRG-RAM bank.
    pub prg_ram_bank: u8,
    /// `$5114-$5117`: PRG banks (8 KiB each).
    pub prg_banks: [u8; 4],
    /// `$5120-$5127`: CHR banks (1 KiB each).
    pub chr_banks: [u8; 8],
    /// `$5128-$512B`: upper 2 bits, two fields per register.
    pub chr_upper: [u8; 4],
    /// `$5130`: upper CHR bits (bits 1-0).
    pub chr_upper2: u8,
    /// `$5203`: scanline IRQ target.
    pub irq_target: u8,
    /// `$5204` bit 7: IRQ enable.
    pub irq_enabled: bool,
    /// `$5204` bit 7: IRQ pending (cleared by reading `$5204`).
    pub irq_pending: bool,
    /// `$5205`: multiplicand.
    pub mul_a: u8,
    /// `$5206`: multiplier.
    pub mul_b: u8,
    /// `$5C00-$5CFF`: 1 KiB of ExRAM.
    pub exram: [u8; 1024],
}

impl Default for Mmc5 {
    fn default() -> Self {
        Self::new()
    }
}

impl Mmc5 {
    /// Power-on state. `$5100`/`$5101` default to mode 3 (8 KiB PRG chunks,
    /// 1 KiB CHR chunks) and every bank register points at bank 0, matching
    /// hardware after reset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            prg_mode: 3,
            chr_mode: 3,
            prg_ram_protect: 0,
            exram_mode: 0,
            nt_map: 0,
            fill_tile: 0,
            fill_attr: 0,
            prg_ram_bank: 0,
            prg_banks: [0; 4],
            chr_banks: [0; 8],
            chr_upper: [0; 4],
            chr_upper2: 0,
            irq_target: 0,
            irq_enabled: false,
            irq_pending: false,
            mul_a: 0,
            mul_b: 0,
            exram: [0; 1024],
        }
    }

    /// Point the power-on window at the **last** banks, like the MMC1 cart
    /// this ROM was translated from.
    ///
    /// # Why this is required
    ///
    /// MMC5 powers up with `$5114-$5117` all zero, which would map bank 0 at
    /// `$E000-$FFFF` and make `$FFFC` read bank 0's `$1FFC`. In this ROM that
    /// byte is `$9658`, and `$9658` inside bank 0 is `$FF` — the machine
    /// would jump into a wall of `$FF` and never draw anything (verified: 0
    /// non-zero pixels after 300 frames).
    ///
    /// The translated image keeps the original's layout: only the **last**
    /// 8 KiB bank carries the real vector table (`NMI=$C07B`, `RESET=$FF4C`,
    /// `IRQ=$FF9C`). So the boot window has to be
    /// `$C000` = second-to-last, `$E000` = last, and `$8000` = bank 0.
    pub fn init_fix_last(&mut self, prg_len: usize) {
        let banks = (prg_len / PRG_BANK_LEN).max(1);
        let last = (banks - 1) as u8;
        let prev = last.saturating_sub(1);
        self.prg_mode = 3;
        self.prg_banks = [0, 0, prev, last];
    }

    /// CHR mode (0 = 8 KiB, 1 = 4 KiB, 2 = 2 KiB, 3 = 1 KiB).
    #[must_use]
    pub fn chr_mode_bits(&self) -> u8 {
        self.chr_mode & 3
    }

    /// PRG mode (0 = 32 KiB, 1 = 16+16, 2 = 16+8+8, 3 = 8x4).
    #[must_use]
    pub fn prg_mode_bits(&self) -> u8 {
        self.prg_mode & 3
    }

    /// Upper 2 bits for 1 KiB CHR bank `slot` (0-7).
    ///
    /// `$5128` holds slots 0 and 1 (bits 0-1 and 2-3), `$5129` holds 2/3, and
    /// so on — two fields per register.
    #[must_use]
    pub fn chr_upper_of(&self, slot: usize) -> u8 {
        let reg = self.chr_upper[(slot >> 1) & 3];
        if slot & 1 == 0 {
            reg & 3
        } else {
            (reg >> 2) & 3
        }
    }

    /// Full 1 KiB CHR bank number for `slot` (0-7), honouring the CHR mode.
    ///
    /// The mode decides which *low* register feeds the slot; the upper bits
    /// always come from `$5128-$512B` plus `$5130`.
    #[must_use]
    pub fn chr_bank_1k(&self, slot: usize) -> usize {
        let slot = slot & 7;
        let low_reg = match self.chr_mode_bits() {
            // 8 KiB: one register ($5127) covers all eight slots.
            0 => 7,
            // 4 KiB: $5123 for $0000-$0FFF, $5127 for $1000-$1FFF.
            1 => {
                if slot < 4 {
                    3
                } else {
                    7
                }
            }
            // 2 KiB: $5121/$5123/$5125/$5127.
            2 => (slot >> 1) * 2 + 1,
            // 1 KiB: one register per slot.
            _ => slot,
        };
        let low = usize::from(self.chr_banks[low_reg]);
        // Upper bits belong to the *register* the slot actually uses.
        let hi = usize::from(self.chr_upper_of(low_reg));
        let hi2 = usize::from(self.chr_upper2 & 3);
        (low | (hi << 8) | (hi2 << 10)) & 0x1FFF
    }

    /// 8 KiB PRG bank number for `slot` (0-3, where slot 0 = `$8000`).
    #[must_use]
    pub fn prg_bank_8k(&self, slot: usize) -> usize {
        let slot = slot & 3;
        let bank = match self.prg_mode_bits() {
            // 32 KiB: $5114, low two bits ignored.
            0 => (self.prg_banks[0] & 0xFC) | slot as u8,
            // 16 KiB at $8000 + 16 KiB at $C000.
            1 => {
                if slot < 2 {
                    (self.prg_banks[0] & 0xFE) | slot as u8
                } else {
                    (self.prg_banks[1] & 0xFE) | (slot as u8 - 2)
                }
            }
            // 16 KiB at $8000, 8 KiB at $C000, 8 KiB at $E000.
            2 => match slot {
                0 | 1 => (self.prg_banks[0] & 0xFE) | slot as u8,
                2 => self.prg_banks[1],
                _ => self.prg_banks[2],
            },
            // Four independent 8 KiB banks.
            _ => self.prg_banks[slot],
        };
        usize::from(bank)
    }

    /// Translate a CPU address in `$8000-$FFFF` to a byte index into the raw
    /// PRG image. Out-of-range banks wrap, like the MMC1 path does.
    ///
    /// # `$E000-$FFFF` is pinned to the last bank
    ///
    /// This is not textbook MMC5 (where `$5116` drives `$E000-$FFFF` in mode
    /// 2), but it is what the translated ROM actually requires. Evidence from
    /// the image itself:
    ///
    /// * `STA $5117` never appears anywhere in PRG — the ROM never uses PRG
    ///   mode 3, only mode 2.
    /// * The boot code at `$FDCB` does `STA $5116` = `$8E` and then **keeps
    ///   executing at `$FDDF`**, i.e. still inside `$E000-$FFFF`. If that
    ///   write really remapped `$E000`, the very next fetch would land in
    ///   bank 14's data area (`$00 $00 …`) and BRK.
    /// * The value it writes (`$8E` = 142, masked to 14) points at a bank
    ///   whose `$C000` **is** real init code
    ///   (`LDA #0 / STA $2001 / STA $2000 / LDA #5 / JSR $FFCC`), and that
    ///   `JSR` target lives in the fixed bank.
    ///
    /// So `$5116` drives `$C000-$DFFF` here, and the top 8 KiB stays put.
    #[must_use]
    pub fn map_prg(&self, addr: u16, prg_len: usize) -> usize {
        debug_assert!(prg_len >= PRG_BANK_LEN);
        let banks = (prg_len / PRG_BANK_LEN).max(1);
        let off = usize::from(addr.wrapping_sub(0x8000));
        // Top 8 KiB: always the last bank.
        if addr >= 0xE000 {
            return (banks - 1) * PRG_BANK_LEN + off % PRG_BANK_LEN;
        }
        let slot = (off / PRG_BANK_LEN) & 3;
        let bank = self.prg_bank_8k(slot);
        (bank % banks) * PRG_BANK_LEN + off % PRG_BANK_LEN
    }

    /// Build the 4 KiB CHR image for pattern-table `page` (0 = `$0000`,
    /// 1 = `$1000`) by concatenating four 1 KiB banks.
    ///
    /// The PPU only knows 4 KiB slots, but MMC5 switches CHR in 1 KiB units,
    /// so the four banks have to be stitched here. Banks that run past the
    /// image leave zeroes (a synthetic small-CHR fixture would otherwise
    /// panic; the real ROM always has enough).
    #[must_use]
    pub fn chr_slot_4k(&self, chr_rom: &[u8], page: u8) -> [u8; CHR_SLOT_LEN] {
        let mut out = [0u8; CHR_SLOT_LEN];
        let banks = (chr_rom.len() / CHR_BANK_LEN).max(1);
        for i in 0..4 {
            let slot = usize::from(page & 1) * 4 + i;
            let bank = self.chr_bank_1k(slot) % banks;
            let src = bank * CHR_BANK_LEN;
            if let Some(data) = chr_rom.get(src..src + CHR_BANK_LEN) {
                out[i * CHR_BANK_LEN..(i + 1) * CHR_BANK_LEN].copy_from_slice(data);
            }
        }
        out
    }

    /// `$5105` decoded: which physical nametable feeds logical screen `i`
    /// (0-3). Value 0-3 = that nametable, 4-7 = ExRAM (not modelled here).
    #[must_use]
    pub fn nametable_source(&self, logical: u8) -> u8 {
        (self.nt_map >> ((logical & 3) * 2)) & 3
    }

    /// Product of the hardware multiplier (`$5205` x `$5206`).
    #[must_use]
    pub fn mul_product(&self) -> u16 {
        u16::from(self.mul_a) * u16::from(self.mul_b)
    }

    /// Called by the PPU once per scanline with the current scanline number;
    /// raises the IRQ when it matches `$5203`.
    pub fn on_scanline(&mut self, line: u16) {
        if self.irq_enabled && line == u16::from(self.irq_target) {
            self.irq_pending = true;
        }
    }

    /// Mapper register write. Returns `true` when the address belonged to
    /// MMC5 (so the caller can stop looking at other mappers).
    pub fn write(&mut self, addr: u16, val: u8) -> bool {
        match addr {
            0x5100 => self.prg_mode = val & 3,
            0x5101 => self.chr_mode = val & 3,
            0x5102 => self.prg_ram_protect = (self.prg_ram_protect & 0x02) | (val & 1),
            0x5103 => self.prg_ram_protect = (self.prg_ram_protect & 0x01) | ((val & 1) << 1),
            0x5104 => self.exram_mode = val & 3,
            0x5105 => self.nt_map = val,
            0x5106 => self.fill_tile = val,
            0x5107 => self.fill_attr = val,
            0x5113 => self.prg_ram_bank = val,
            0x5114..=0x5117 => self.prg_banks[usize::from(addr - 0x5114)] = val,
            0x5120..=0x5127 => self.chr_banks[usize::from(addr - 0x5120)] = val,
            0x5128..=0x512B => self.chr_upper[usize::from(addr - 0x5128)] = val,
            0x5130 => self.chr_upper2 = val & 3,
            0x5203 => self.irq_target = val,
            0x5204 => {
                self.irq_enabled = val & 0x80 != 0;
                if !self.irq_enabled {
                    self.irq_pending = false;
                }
            }
            0x5205 => self.mul_a = val,
            0x5206 => self.mul_b = val,
            0x5C00..=0x5FFF => self.exram[usize::from(addr - 0x5C00)] = val,
            _ => return false,
        }
        true
    }

    /// Mapper register read. Returns `None` for addresses MMC5 doesn't own.
    #[must_use]
    pub fn read(&mut self, addr: u16) -> Option<u8> {
        match addr {
            0x5100 => Some(self.prg_mode),
            0x5101 => Some(self.chr_mode),
            0x5102 | 0x5103 => Some(self.prg_ram_protect),
            0x5104 => Some(self.exram_mode),
            0x5105 => Some(self.nt_map),
            0x5106 => Some(self.fill_tile),
            0x5107 => Some(self.fill_attr),
            0x5113 => Some(self.prg_ram_bank),
            0x5114..=0x5117 => Some(self.prg_banks[usize::from(addr - 0x5114)]),
            0x5120..=0x5127 => Some(self.chr_banks[usize::from(addr - 0x5120)]),
            0x5128..=0x512B => Some(self.chr_upper[usize::from(addr - 0x5128)]),
            0x5130 => Some(self.chr_upper2),
            0x5203 => Some(self.irq_target),
            0x5204 => {
                // Bit 7 = pending, bit 6 = in-frame; reading clears pending.
                let v = (u8::from(self.irq_pending) << 7) | (u8::from(self.irq_enabled) << 6);
                self.irq_pending = false;
                Some(v)
            }
            0x5205 => Some(self.mul_product() as u8),
            0x5206 => Some((self.mul_product() >> 8) as u8),
            0x5C00..=0x5FFF => Some(self.exram[usize::from(addr - 0x5C00)]),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prg_mode3_maps_four_independent_8k_banks() {
        let mut m = Mmc5::new();
        m.prg_mode = 3;
        m.prg_banks = [1, 2, 3, 4];
        let len = 32 * PRG_BANK_LEN; // 256 KiB
        assert_eq!(m.map_prg(0x8000, len), PRG_BANK_LEN);
        assert_eq!(m.map_prg(0xA000, len), 2 * PRG_BANK_LEN);
        assert_eq!(m.map_prg(0xC000, len), 3 * PRG_BANK_LEN);
        // `$E000-$FFFF` is pinned to the last bank, not `$5117` — see
        // `map_prg` docs for why the translated ROM needs this.
        assert_eq!(m.map_prg(0xE000, len), 31 * PRG_BANK_LEN);
    }

    #[test]
    fn prg_mode0_maps_32k_window() {
        let mut m = Mmc5::new();
        m.prg_mode = 0;
        m.prg_banks[0] = 5; // low two bits ignored -> base 4
        let len = 32 * PRG_BANK_LEN;
        assert_eq!(m.map_prg(0x8000, len), 4 * PRG_BANK_LEN);
        assert_eq!(m.map_prg(0xA000, len), 5 * PRG_BANK_LEN);
        assert_eq!(m.map_prg(0xC000, len), 6 * PRG_BANK_LEN);
        // Pinned to the last bank (see `map_prg` docs).
        assert_eq!(m.map_prg(0xE000, len), 31 * PRG_BANK_LEN);
    }

    #[test]
    fn prg_mode1_has_two_16k_windows() {
        let mut m = Mmc5::new();
        m.prg_mode = 1;
        m.prg_banks[0] = 2;
        m.prg_banks[1] = 10;
        let len = 32 * PRG_BANK_LEN;
        assert_eq!(m.map_prg(0x8000, len), 2 * PRG_BANK_LEN);
        assert_eq!(m.map_prg(0xA000, len), 3 * PRG_BANK_LEN);
        assert_eq!(m.map_prg(0xC000, len), 10 * PRG_BANK_LEN);
        // Pinned to the last bank (see `map_prg` docs).
        assert_eq!(m.map_prg(0xE000, len), 31 * PRG_BANK_LEN);
    }

    #[test]
    fn chr_mode3_uses_one_register_per_1k_slot() {
        let mut m = Mmc5::new();
        m.chr_mode = 3;
        for i in 0..8 {
            m.chr_banks[i] = 0x10 + i as u8;
        }
        for i in 0..8 {
            assert_eq!(m.chr_bank_1k(i), 0x10 + i);
        }
    }

    #[test]
    fn chr_mode0_uses_only_5127() {
        let mut m = Mmc5::new();
        m.chr_mode = 0;
        m.chr_banks[7] = 0x20;
        for i in 0..8 {
            // 8 KiB mode: one base, low three bits of the register ignored.
            assert_eq!(m.chr_bank_1k(i), 0x20);
        }
    }

    #[test]
    fn chr_upper_bits_extend_bank_number() {
        let mut m = Mmc5::new();
        m.chr_mode = 3;
        m.chr_banks[0] = 0x73;
        // $5128 bits 0-1 apply to $5120.
        m.chr_upper[0] = 0x01;
        assert_eq!(m.chr_bank_1k(0), 0x173);
    }

    #[test]
    fn chr_slot_stitches_four_1k_banks() {
        let mut m = Mmc5::new();
        m.chr_mode = 3;
        m.chr_banks = [0, 1, 2, 3, 4, 5, 6, 7];
        // 8 KiB image = 8 banks of 1 KiB, bank N filled with N.
        let mut rom = vec![0u8; 8 * CHR_BANK_LEN];
        for bank in 0..8 {
            for b in &mut rom[bank * CHR_BANK_LEN..(bank + 1) * CHR_BANK_LEN] {
                *b = bank as u8;
            }
        }
        let slot0 = m.chr_slot_4k(&rom, 0);
        assert_eq!(slot0[0], 0);
        assert_eq!(slot0[CHR_BANK_LEN], 1);
        assert_eq!(slot0[3 * CHR_BANK_LEN], 3);
        let slot1 = m.chr_slot_4k(&rom, 1);
        assert_eq!(slot1[0], 4);
        assert_eq!(slot1[3 * CHR_BANK_LEN], 7);
    }

    #[test]
    fn multiplier_reads_low_then_high() {
        let mut m = Mmc5::new();
        m.write(0x5205, 0x10);
        m.write(0x5206, 0x20);
        assert_eq!(m.read(0x5205), Some(0x00));
        assert_eq!(m.read(0x5206), Some(0x02));
    }

    #[test]
    fn irq_fires_on_matching_scanline_and_read_clears() {
        let mut m = Mmc5::new();
        m.write(0x5203, 100);
        m.write(0x5204, 0x80);
        m.on_scanline(99);
        assert!(!m.irq_pending);
        m.on_scanline(100);
        assert!(m.irq_pending);
        assert_eq!(m.read(0x5204).map(|v| v & 0x80), Some(0x80));
        assert!(!m.irq_pending, "读 $5204 应清 pending");
    }

    #[test]
    fn init_fix_last_puts_vectors_in_place() {
        let mut m = Mmc5::new();
        let prg_len = 32 * PRG_BANK_LEN; // 256 KiB
        m.init_fix_last(prg_len);
        assert_eq!(m.prg_mode, 3);
        assert_eq!(m.prg_bank_8k(0), 0, "$8000 用 bank 0");
        assert_eq!(m.prg_bank_8k(1), 0, "$A000 用 bank 0");
        assert_eq!(m.prg_bank_8k(2), 30, "$C000 用倒数第二个");
        assert_eq!(m.prg_bank_8k(3), 31, "$E000 用最后一个（向量表在这）");
    }

    #[test]
    fn nametable_map_decodes_two_bits_per_screen() {
        let mut m = Mmc5::new();
        m.write(0x5105, 0b11_10_01_00);
        assert_eq!(m.nametable_source(0), 0);
        assert_eq!(m.nametable_source(1), 1);
        assert_eq!(m.nametable_source(2), 2);
        assert_eq!(m.nametable_source(3), 3);
    }
}
