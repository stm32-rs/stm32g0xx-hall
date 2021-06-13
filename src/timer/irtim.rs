use crate::gpio::gpiob;
use crate::gpio::Analog;
use crate::rcc::Rcc;
use crate::stm32;
use crate::time::Hertz;

pub trait IrOutPin {
    fn setup(&self);
}

pub trait IrModulator {
    fn setup(&self) -> u8;
}

pub trait IrTransmitterExt<M, IR>: Sized
where
    M: IrModulator,
    IR: IrOutPin,
{
    fn ir_transmitter<T>(
        self,
        freq: T,
        modulator: M,
        ir_out: IR,
        rcc: &mut Rcc,
    ) -> IrTransmitter<M, IR>
    where
        IR: IrOutPin,
        T: Into<Hertz>;
}

pub struct IrTransmitter<M, IR>
where
    M: IrModulator,
    IR: IrOutPin,
{
    _ir_out: IR,
    _modulator: M,
}

impl<M, IR> IrTransmitter<M, IR>
where
    M: IrModulator,
    IR: IrOutPin,
{
    pub fn new<T>(carrier: stm32::TIM17, freq: T, _modulator: M, _ir_out: IR, rcc: &mut Rcc) -> Self
    where
        T: Into<Hertz>,
    {
        rcc.rb.apbenr2.modify(|_, w| w.tim17en().set_bit());
        rcc.rb.apbrstr2.modify(|_, w| w.tim17rst().set_bit());
        rcc.rb.apbrstr2.modify(|_, w| w.tim17rst().clear_bit());
        let ratio = rcc.clocks.apb_tim_clk / freq.into();
        let psc = (ratio - 1) / 0xffff;
        let arr = ratio / (psc + 1) - 1;
        unsafe {
            carrier.psc.write(|w| w.psc().bits(psc as u16));
            carrier.arr.write(|w| w.arr().bits(arr as u16));
            carrier.ccr1.write(|w| w.bits(arr / 2));
        }
        carrier.cr1.write(|w| w.cen().set_bit());

        _ir_out.setup();
        // TODO: fix SVD file
        _modulator.setup();

        Self {
            _ir_out,
            _modulator,
        }
    }
}

impl<M, IR> hal::PwmPin for IrTransmitter<M, IR>
where
    M: IrModulator,
    IR: IrOutPin,
{
    type Duty = u32;

    fn disable(&mut self) {
        unsafe {
            (*stm32::TIM17::ptr())
                .ccer
                .modify(|_, w| w.cc1e().clear_bit());
        }
    }

    fn enable(&mut self) {
        unsafe {
            let tim = &*stm32::TIM17::ptr();
            tim.ccmr1_output()
                .modify(|_, w| w.oc1pe().set_bit().oc1m().bits(6));
            tim.ccer.modify(|_, w| w.cc1e().set_bit());
        }
    }

    fn get_duty(&self) -> u32 {
        unsafe { (*stm32::TIM17::ptr()).ccr1.read().bits() }
    }

    fn get_max_duty(&self) -> u32 {
        unsafe { (*stm32::TIM17::ptr()).arr.read().bits() }
    }

    fn set_duty(&mut self, duty: u32) {
        unsafe { (*stm32::TIM17::ptr()).ccr1.write(|w| w.bits(duty)) }
    }
}

impl IrOutPin for gpiob::PB9<Analog> {
    fn setup(&self) {
        todo!();
        // ir_out.set_alt_mode(AltFunction::AF0);
    }
}

impl IrModulator for stm32::TIM16 {
    fn setup(&self) -> u8 {
        0
    }
}

impl<M, IR> IrTransmitterExt<M, IR> for stm32::TIM17
where
    M: IrModulator,
    IR: IrOutPin,
{
    fn ir_transmitter<T>(
        self,
        freq: T,
        modulator: M,
        ir_out: IR,
        rcc: &mut Rcc,
    ) -> IrTransmitter<M, IR>
    where
        M: IrModulator,
        IR: IrOutPin,
        T: Into<Hertz>,
    {
        IrTransmitter::new(self, freq, modulator, ir_out, rcc)
    }
}
