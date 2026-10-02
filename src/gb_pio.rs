/* RP2350 GameBoy cartridge
 * Copyright (C) 2025 Sebastian Quilitz
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::dma_helper::{DmaReadTarget, DmaWriteTarget};
use embassy_rp::{
    pac,
    pio::{
        program::pio_file, Common, Config, Direction, ExecConfig, FifoJoin, Instance, Pin,
        PinConfig, PioPin, ShiftConfig, ShiftDirection, StateMachine, StateMachineRx,
    },
    Peri,
};

pub struct GbPioPins<'d, P: Instance> {
    pub _addr_pins: [Pin<'d, P>; 16],
    pub _data_pins: [Pin<'d, P>; 8],
    pub _ctrl_pins: [Pin<'d, P>; 3],
    pub debug_pin: Option<Pin<'d, P>>,
}
impl<'d, P: Instance> GbPioPins<'d, P> {
    pub fn new(
        pio: &mut Common<'d, P>,
        clk_pin: Peri<'d, impl PioPin>,
        rd_pin: Peri<'d, impl PioPin>,
        cs_pin: Peri<'d, impl PioPin>,
        a0_pin: Peri<'d, impl PioPin>,
        a1_pin: Peri<'d, impl PioPin>,
        a2_pin: Peri<'d, impl PioPin>,
        a3_pin: Peri<'d, impl PioPin>,
        a4_pin: Peri<'d, impl PioPin>,
        a5_pin: Peri<'d, impl PioPin>,
        a6_pin: Peri<'d, impl PioPin>,
        a7_pin: Peri<'d, impl PioPin>,
        a8_pin: Peri<'d, impl PioPin>,
        a9_pin: Peri<'d, impl PioPin>,
        a10_pin: Peri<'d, impl PioPin>,
        a11_pin: Peri<'d, impl PioPin>,
        a12_pin: Peri<'d, impl PioPin>,
        a13_pin: Peri<'d, impl PioPin>,
        a14_pin: Peri<'d, impl PioPin>,
        a15_pin: Peri<'d, impl PioPin>,
        d0_pin: Peri<'d, impl PioPin>,
        d1_pin: Peri<'d, impl PioPin>,
        d2_pin: Peri<'d, impl PioPin>,
        d3_pin: Peri<'d, impl PioPin>,
        d4_pin: Peri<'d, impl PioPin>,
        d5_pin: Peri<'d, impl PioPin>,
        d6_pin: Peri<'d, impl PioPin>,
        d7_pin: Peri<'d, impl PioPin>,
        debug_pin: Option<Peri<'d, impl PioPin>>,
    ) -> Self {
        let mut ctrl_pins: [embassy_rp::pio::Pin<'_, P>; 3] = [
            pio.make_pio_pin(clk_pin),
            pio.make_pio_pin(rd_pin),
            pio.make_pio_pin(cs_pin),
        ];

        let mut addr_pins: [embassy_rp::pio::Pin<'_, P>; 16] = [
            pio.make_pio_pin(a0_pin),
            pio.make_pio_pin(a1_pin),
            pio.make_pio_pin(a2_pin),
            pio.make_pio_pin(a3_pin),
            pio.make_pio_pin(a4_pin),
            pio.make_pio_pin(a5_pin),
            pio.make_pio_pin(a6_pin),
            pio.make_pio_pin(a7_pin),
            pio.make_pio_pin(a8_pin),
            pio.make_pio_pin(a9_pin),
            pio.make_pio_pin(a10_pin),
            pio.make_pio_pin(a11_pin),
            pio.make_pio_pin(a12_pin),
            pio.make_pio_pin(a13_pin),
            pio.make_pio_pin(a14_pin),
            pio.make_pio_pin(a15_pin),
        ];

        let mut data_pins = [
            pio.make_pio_pin(d0_pin),
            pio.make_pio_pin(d1_pin),
            pio.make_pio_pin(d2_pin),
            pio.make_pio_pin(d3_pin),
            pio.make_pio_pin(d4_pin),
            pio.make_pio_pin(d5_pin),
            pio.make_pio_pin(d6_pin),
            pio.make_pio_pin(d7_pin),
        ];

        for pin in &mut ctrl_pins {
            pac::PADS_BANK0.gpio(pin.pin() as usize).modify(|w| {
                w.set_ie(true);
            })
        }

        for pin in &mut addr_pins {
            pac::PADS_BANK0.gpio(pin.pin() as usize).modify(|w| {
                w.set_ie(true);
            })
        }

        for pin in &mut data_pins {
            pac::PADS_BANK0.gpio(pin.pin() as usize).modify(|w| {
                w.set_ie(true);
            })
        }

        let debug_pin = match debug_pin {
            Some(pin) => {
                let pin = pio.make_pio_pin(pin);
                Some(pin)
            }
            None => None,
        };

        Self {
            _ctrl_pins: ctrl_pins,
            _addr_pins: addr_pins,
            _data_pins: data_pins,
            debug_pin,
        }
    }
}

pub struct GbDataOut<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
    p: &'static pac::pio::Pio,
}

impl<'d, P: Instance, const S: usize> GbDataOut<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        p: &'static pac::pio::Pio,
        mut sm: StateMachine<'d, P, S>,
        pins: &GbPioPins<'d, P>,
    ) -> Self {
        let program = pio_file!(
            "./pio/gameboy_bus.pio",
            select_program("gameboy_bus_write_to_data"),
            options(max_program_size = 32) // Optional, defaults to 32
        );
        let mut cfg = Config::default();
        let mut pincfg = PinConfig::default();
        pincfg.in_base = program.public_defines.pin_clk as u8;
        pincfg.out_base = program.public_defines.pin_data_base as u8;
        pincfg.out_count = 8;
        unsafe {
            cfg.set_pins(pincfg);
        }
        cfg.shift_out = ShiftConfig {
            auto_fill: false,
            threshold: 8,
            direction: ShiftDirection::Right,
        };
        cfg.use_program(&pio.load_program(&program.program), &[]);

        sm.set_config(&cfg);

        match &pins.debug_pin {
            Some(pin) => {
                sm.set_pin_dirs(Direction::Out, &[&pin]);
            }
            _ => {}
        }

        Self { sm, p }
    }

    pub fn start(&mut self) {
        self.sm.set_enable(true);
    }
}

unsafe impl<'d, P: Instance, const S: usize> DmaWriteTarget for GbDataOut<'d, P, S> {
    type TransmittedWord = u8;

    fn tx_treq(&self) -> Option<u8> {
        let pio_num: u8 = ((self.p.as_ptr() as u32 - pac::PIO0.as_ptr() as u32) / 0x10_0000u32)
            .try_into()
            .unwrap();

        Some(pio_num * 8 + S as u8)
    }

    fn tx_address_count(&self) -> (u32, u32) {
        let ptr = self.p.txf(S).as_ptr();
        (ptr as u32, 1u32)
    }

    fn tx_increment(&self) -> bool {
        false
    }
}

pub struct GbRomDetect<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
}

impl<'d, P: Instance, const S: usize> GbRomDetect<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        _p: &'static pac::pio::Pio,
        mut sm: StateMachine<'d, P, S>,
    ) -> Self {
        let program = pio_file!(
            "./pio/gameboy_bus.pio",
            select_program("gameboy_bus_detect_a15_low_a14_irqs"),
            options(max_program_size = 32) // Optional, defaults to 32
        );
        let mut cfg = Config::default();
        let mut pincfg = PinConfig::default();
        pincfg.in_base = program.public_defines.pin_a15 as u8;
        pincfg.out_base = program.public_defines.pin_data_base as u8;
        pincfg.out_count = 8;
        unsafe {
            cfg.set_pins(pincfg);
        }
        let mut execcfg = ExecConfig::default();
        execcfg.jmp_pin = program.public_defines.pin_a14 as u8 - 16; // todo: offset should be calculated automatically
        unsafe {
            cfg.set_exec(execcfg);
        }
        cfg.shift_in = ShiftConfig {
            auto_fill: false,
            threshold: 32,
            direction: ShiftDirection::Right,
        };
        let mut lprogram = pio.load_program(&program.program);
        lprogram.origin += program.public_defines.entry_point as u8; // offset the start
        cfg.use_program(&lprogram, &[]);

        sm.set_config(&cfg);

        Self { sm }
    }

    pub fn start(&mut self) {
        self.sm.set_enable(true);
    }
}

pub struct GbRomLower<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
    p: &'static pac::pio::Pio,
}
impl<'d, P: Instance, const S: usize> GbRomLower<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        p: &'static pac::pio::Pio,
        mut sm: StateMachine<'d, P, S>,
        _pins: &GbPioPins<'d, P>,
    ) -> Self {
        let program = pio_file!(
            "./pio/gameboy_bus.pio",
            select_program("gameboy_bus_rom_low"),
            options(max_program_size = 32) // Optional, defaults to 32
        );
        let mut cfg = Config::default();
        let mut pincfg = PinConfig::default();
        pincfg.in_base = program.public_defines.pin_ad_base as u8;
        pincfg.out_base = program.public_defines.pin_data_base as u8;
        pincfg.out_count = 8;
        unsafe {
            cfg.set_pins(pincfg);
        }
        let mut execcfg = ExecConfig::default();
        execcfg.jmp_pin = program.public_defines.pin_rd as u8 - 16; // todo: offset should be calculated automatically
        unsafe {
            cfg.set_exec(execcfg);
        }
        cfg.shift_in = ShiftConfig {
            auto_fill: true,
            threshold: 14,
            direction: ShiftDirection::Left,
        };
        cfg.use_program(&pio.load_program(&program.program), &[]);

        sm.set_config(&cfg);

        Self { sm, p }
    }

    pub fn start(&mut self) {
        self.sm.set_enable(true);
    }
}

unsafe impl<'d, P: Instance, const S: usize> DmaReadTarget for GbRomLower<'d, P, S> {
    type ReceivedWord = u32;

    fn rx_treq(&self) -> Option<u8> {
        let pio_num: u8 = ((self.p.as_ptr() as u32 - pac::PIO0.as_ptr() as u32) / 0x10_0000u32)
            .try_into()
            .unwrap();

        Some(pio_num * 8 + 4 + S as u8)
    }

    fn rx_address_count(&self) -> (u32, u32) {
        let ptr = self.p.rxf(S).as_ptr();
        (ptr as u32, 1u32)
    }

    fn rx_increment(&self) -> bool {
        false
    }
}

pub struct GbRomHigher<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
    p: &'static pac::pio::Pio,
}
impl<'d, P: Instance, const S: usize> GbRomHigher<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        p: &'static pac::pio::Pio,
        mut sm: StateMachine<'d, P, S>,
        _pins: &GbPioPins<'d, P>,
    ) -> Self {
        let program = pio_file!(
            "./pio/gameboy_bus.pio",
            select_program("gameboy_bus_rom_high"),
            options(max_program_size = 32) // Optional, defaults to 32
        );
        let mut cfg = Config::default();
        let mut pincfg = PinConfig::default();
        pincfg.in_base = program.public_defines.pin_ad_base as u8;
        pincfg.out_base = program.public_defines.pin_data_base as u8;
        pincfg.out_count = 8;
        unsafe {
            cfg.set_pins(pincfg);
        }
        let mut execcfg = ExecConfig::default();
        execcfg.jmp_pin = program.public_defines.pin_rd as u8 - 16; // todo: offset should be calculated automatically
        unsafe {
            cfg.set_exec(execcfg);
        }
        cfg.shift_in = ShiftConfig {
            auto_fill: true,
            threshold: 14,
            direction: ShiftDirection::Left,
        };
        cfg.use_program(&pio.load_program(&program.program), &[]);

        sm.set_config(&cfg);

        Self { sm, p }
    }

    pub fn start(&mut self) {
        self.sm.set_enable(true);
    }
}

unsafe impl<'d, P: Instance, const S: usize> DmaReadTarget for GbRomHigher<'d, P, S> {
    type ReceivedWord = u32;

    fn rx_treq(&self) -> Option<u8> {
        let pio_num: u8 = ((self.p.as_ptr() as u32 - pac::PIO0.as_ptr() as u32) / 0x10_0000u32)
            .try_into()
            .unwrap();

        Some(pio_num * 8 + 4 + S as u8)
    }

    fn rx_address_count(&self) -> (u32, u32) {
        let ptr = self.p.rxf(S).as_ptr();
        (ptr as u32, 1u32)
    }

    fn rx_increment(&self) -> bool {
        false
    }
}

pub struct GbRamRead<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
    p: &'static pac::pio::Pio,
}
impl<'d, P: Instance, const S: usize> GbRamRead<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        p: &'static pac::pio::Pio,
        mut sm: StateMachine<'d, P, S>,
    ) -> Self {
        let program = pio_file!(
            "./pio/gameboy_bus.pio",
            select_program("gameboy_saveram_read"),
            options(max_program_size = 32) // Optional, defaults to 32
        );
        let mut cfg = Config::default();
        let mut pincfg = PinConfig::default();
        pincfg.in_base = program.public_defines.pin_ad_base as u8;
        pincfg.out_base = program.public_defines.pin_data_base as u8;
        pincfg.out_count = 8;
        unsafe {
            cfg.set_pins(pincfg);
        }
        let mut execcfg = ExecConfig::default();
        execcfg.jmp_pin = program.public_defines.pin_rd as u8 - 16; // todo: offset should be calculated automatically
        unsafe {
            cfg.set_exec(execcfg);
        }
        cfg.shift_in = ShiftConfig {
            auto_fill: false,
            threshold: 32,
            direction: ShiftDirection::Right,
        };

        let mut lprogram = pio.load_program(&program.program);
        lprogram.origin += program.public_defines.entry_point as u8; // offset the start
        cfg.use_program(&lprogram, &[]);

        sm.set_config(&cfg);

        Self { sm, p }
    }

    pub fn start(&mut self) {
        self.sm.set_enable(true);
    }
}

unsafe impl<'d, P: Instance, const S: usize> DmaReadTarget for GbRamRead<'d, P, S> {
    type ReceivedWord = u32;

    fn rx_treq(&self) -> Option<u8> {
        let pio_num: u8 = ((self.p.as_ptr() as u32 - pac::PIO0.as_ptr() as u32) / 0x10_0000u32)
            .try_into()
            .unwrap();

        Some(pio_num * 8 + 4 + S as u8)
    }

    fn rx_address_count(&self) -> (u32, u32) {
        let ptr = self.p.rxf(S).as_ptr();
        (ptr as u32, 1u32)
    }

    fn rx_increment(&self) -> bool {
        false
    }
}

pub struct GbRamWrite<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
    p: &'static pac::pio::Pio,
}
impl<'d, P: Instance, const S: usize> GbRamWrite<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        p: &'static pac::pio::Pio,
        mut sm: StateMachine<'d, P, S>,
    ) -> Self {
        let program = pio_file!(
            "./pio/gameboy_bus.pio",
            select_program("gameboy_saveram_write"),
            options(max_program_size = 32) // Optional, defaults to 32
        );
        let mut cfg = Config::default();
        let mut pincfg = PinConfig::default();
        pincfg.in_base = program.public_defines.pin_ad_base as u8;
        pincfg.out_base = program.public_defines.pin_data_base as u8;
        pincfg.out_count = 8;
        unsafe {
            cfg.set_pins(pincfg);
        }
        let mut execcfg = ExecConfig::default();
        execcfg.jmp_pin = program.public_defines.pin_rd as u8 - 16; // todo: offset should be calculated automatically
        unsafe {
            cfg.set_exec(execcfg);
        }
        cfg.shift_in = ShiftConfig {
            auto_fill: true,
            threshold: 32,
            direction: ShiftDirection::Right,
        };

        cfg.use_program(&pio.load_program(&program.program), &[]);

        sm.set_config(&cfg);

        Self { sm, p }
    }

    pub fn start(&mut self) {
        self.sm.set_enable(true);
    }
}

unsafe impl<'d, P: Instance, const S: usize> DmaReadTarget for GbRamWrite<'d, P, S> {
    type ReceivedWord = u32;

    fn rx_treq(&self) -> Option<u8> {
        let pio_num: u8 = ((self.p.as_ptr() as u32 - pac::PIO0.as_ptr() as u32) / 0x10_0000u32)
            .try_into()
            .unwrap();

        Some(pio_num * 8 + 4 + S as u8)
    }

    fn rx_address_count(&self) -> (u32, u32) {
        let ptr = self.p.rxf(S).as_ptr();
        (ptr as u32, 1u32)
    }

    fn rx_increment(&self) -> bool {
        false
    }
}

pub struct GbMbcCommands<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
    _p: &'static pac::pio::Pio,
}
impl<'d, P: Instance, const S: usize> GbMbcCommands<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        p: &'static pac::pio::Pio,
        mut sm: StateMachine<'d, P, S>,
    ) -> Self {
        let program = pio_file!(
            "./pio/gameboy_bus.pio",
            select_program("gameboy_mbc_commands"),
            options(max_program_size = 32) // Optional, defaults to 32
        );
        let mut cfg = Config::default();
        let mut pincfg = PinConfig::default();
        pincfg.in_base = program.public_defines.pin_ad_base as u8;
        pincfg.out_base = program.public_defines.pin_data_base as u8;
        pincfg.out_count = 8;
        unsafe {
            cfg.set_pins(pincfg);
        }
        let mut execcfg = ExecConfig::default();
        execcfg.jmp_pin = program.public_defines.pin_rd as u8 - 16; // todo: offset should be calculated automatically
        unsafe {
            cfg.set_exec(execcfg);
        }
        cfg.shift_in = ShiftConfig {
            auto_fill: true,
            threshold: 32,
            direction: ShiftDirection::Right,
        };

        let mut lprogram = pio.load_program(&program.program);
        lprogram.origin += program.public_defines.entry_point as u8; // offset the start
        cfg.use_program(&lprogram, &[]);

        sm.set_config(&cfg);

        Self { sm, _p: p }
    }

    pub fn start(&mut self) {
        self.sm.set_enable(true);
    }

    pub fn rx_fifo(&mut self) -> &mut StateMachineRx<'d, P, S> {
        self.sm.rx()
    }
}

/// Records every bus cycle with CS low (0xA000 to 0xFDFF), reads and writes alike, as one word
/// per cycle: A0 to A15 in bits 0-15, D0 to D7 in bits 16-23. Used to keep a mirror of the
/// work RAM of the running game.
pub struct GbBusMirror<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
    p: &'static pac::pio::Pio,
}
impl<'d, P: Instance, const S: usize> GbBusMirror<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        p: &'static pac::pio::Pio,
        mut sm: StateMachine<'d, P, S>,
    ) -> Self {
        let program = pio_file!(
            "./pio/gameboy_bus.pio",
            select_program("gameboy_bus_mirror"),
            options(max_program_size = 32) // Optional, defaults to 32
        );
        let mut cfg = Config::default();
        let mut pincfg = PinConfig::default();
        pincfg.in_base = program.public_defines.pin_ad_base as u8;
        unsafe {
            cfg.set_pins(pincfg);
        }
        cfg.shift_in = ShiftConfig {
            auto_fill: true,
            threshold: 24,
            direction: ShiftDirection::Left,
        };
        // only the RX direction is used, this gives a deeper FIFO
        cfg.fifo_join = FifoJoin::RxOnly;
        cfg.use_program(&pio.load_program(&program.program), &[]);

        sm.set_config(&cfg);

        Self { sm, p }
    }

    pub fn start(&mut self) {
        self.sm.set_enable(true);
    }

    /// Returns true if the state machine had to wait for space in its RX FIFO since the last
    /// call. Bus cycles happening during such a stall are missing from the mirror.
    pub fn take_stalled(p: &'static pac::pio::Pio) -> bool {
        let stalled = p.fdebug().read().rxstall() & (1u8 << S) != 0;
        if stalled {
            p.fdebug().write(|w| w.set_rxstall(1u8 << S));
        }
        stalled
    }
}

unsafe impl<'d, P: Instance, const S: usize> DmaReadTarget for GbBusMirror<'d, P, S> {
    type ReceivedWord = u32;

    fn rx_treq(&self) -> Option<u8> {
        let pio_num: u8 = ((self.p.as_ptr() as u32 - pac::PIO0.as_ptr() as u32) / 0x10_0000u32)
            .try_into()
            .unwrap();

        Some(pio_num * 8 + 4 + S as u8)
    }

    fn rx_address_count(&self) -> (u32, u32) {
        let ptr = self.p.rxf(S).as_ptr();
        (ptr as u32, 1u32)
    }

    fn rx_increment(&self) -> bool {
        false
    }
}
