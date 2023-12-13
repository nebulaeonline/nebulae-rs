#![cfg(target_arch = "x86_64")]

use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::serial_println;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_error.set_handler_fn(div0_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(nmi_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_na_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_np_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.x87_floating_point.set_handler_fn(x87_fp_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.simd_floating_point.set_handler_fn(simd_fp_handler);
        idt.virtualization.set_handler_fn(virtualization_fault_handler);
        idt.security_exception.set_handler_fn(security_handler);

        idt
    };
}

pub fn exceptions_init() {
    IDT.load();
}

extern "x86-interrupt" fn div0_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: DIVIDE BY ZERO\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: DEBUG\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: NMI\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn bound_range_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn device_na_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, data: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT ({})\n{:#?}", data, stack_frame);
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, data: u64) {
    serial_println!("EXCEPTION: INVALID TSS ({})\n{:#?}", data, stack_frame);
}

extern "x86-interrupt" fn segment_np_handler(stack_frame: InterruptStackFrame, data: u64) {
    serial_println!("EXCEPTION: SEGMENT NOT PRESENT FAULT ({})\n{:#?}", data, stack_frame);
}

extern "x86-interrupt" fn stack_segment_handler(stack_frame: InterruptStackFrame, data: u64) {
    serial_println!("EXCEPTION: STACK SEGMENT FAULT ({})\n{:#?}", data, stack_frame);
}

extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, data: u64) {
    serial_println!("EXCEPTION: GENERAL PROTECTION FAULT ({})\n{:#?}", data, stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, page_fault_data: PageFaultErrorCode) {
    panic!("EXCEPTION: PAGE FAULT ({:?})\n{:#?}", page_fault_data, stack_frame);
}

extern "x86-interrupt" fn x87_fp_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: x87 FLOATING POINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, data: u64) {
    serial_println!("EXCEPTION: ALIGNMENT CHECK ({})\n{:#?}", data, stack_frame);
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    panic!("EXCEPTION: MACHINE CHECK\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn simd_fp_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: SIMD FLOATING POINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn virtualization_fault_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: VIRTUALIZATION FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn security_handler(stack_frame: InterruptStackFrame, data: u64) {
    serial_println!("EXCEPTION: SECURITY ({})\n{:#?}", data, stack_frame);
}