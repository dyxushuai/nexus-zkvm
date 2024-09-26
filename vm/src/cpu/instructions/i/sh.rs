use crate::cpu::instructions::macros::implement_store_instruction;
use crate::{
    cpu::{
        registerfile::RegisterFile,
        state::{Cpu, InstructionExecutor, InstructionState},
    },
    error::{Result, VMError},
    memory::{MemAccessSize, MemoryProcessor},
    riscv::Instruction,
};

pub struct ShInstruction {
    rd: u32,
    rs1: u32,
    imm: u32,
}

implement_store_instruction!(ShInstruction, MemAccessSize::HalfWord);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::state::Cpu;
    use crate::memory::VariableMemory;
    use crate::riscv::{BuiltinOpcode, Instruction, InstructionType, Opcode, Register};

    fn setup_memory() -> VariableMemory {
        VariableMemory::default()
    }

    #[test]
    fn test_sh_positive_value() {
        let mut cpu = Cpu::default();
        let mut memory = setup_memory();

        cpu.registers.write(Register::X1, 0x1000);
        cpu.registers.write(Register::X2, 0x7FFF);

        let bare_instruction = Instruction::new(
            Opcode::from(BuiltinOpcode::SH),
            2,
            1,
            0,
            InstructionType::SType,
        );
        let instruction = ShInstruction::decode(&bare_instruction, &cpu.registers);

        instruction.memory_write(&mut memory).unwrap();

        assert_eq!(
            memory.read(0x1000, MemAccessSize::HalfWord).unwrap(),
            0x7FFF
        );
    }

    #[test]
    fn test_sh_negative_value() {
        let mut cpu = Cpu::default();
        let mut memory = setup_memory();

        cpu.registers.write(Register::X1, 0x1000);
        cpu.registers.write(Register::X2, 0xFFFF8000); // -32768 in two's complement

        let bare_instruction = Instruction::new(
            Opcode::from(BuiltinOpcode::SH),
            2,
            1,
            2,
            InstructionType::SType,
        );
        let instruction = ShInstruction::decode(&bare_instruction, &cpu.registers);

        instruction.memory_write(&mut memory).unwrap();

        assert_eq!(
            memory.read(0x1002, MemAccessSize::HalfWord).unwrap(),
            0x8000
        );
    }

    #[test]
    fn test_sh_max_value() {
        let mut cpu = Cpu::default();
        let mut memory = setup_memory();

        cpu.registers.write(Register::X1, 0x1000);
        cpu.registers.write(Register::X2, 0xFFFF);

        let bare_instruction = Instruction::new(
            Opcode::from(BuiltinOpcode::SH),
            2,
            1,
            4,
            InstructionType::SType,
        );
        let instruction = ShInstruction::decode(&bare_instruction, &cpu.registers);

        instruction.memory_write(&mut memory).unwrap();

        assert_eq!(
            memory.read(0x1004, MemAccessSize::HalfWord).unwrap(),
            0xFFFF
        );
    }

    #[test]
    fn test_sh_unaligned_address() {
        let mut cpu = Cpu::default();
        let mut memory = setup_memory();

        cpu.registers.write(Register::X1, 0x1001); // Unaligned address
        cpu.registers.write(Register::X2, 0xABCD);

        let bare_instruction = Instruction::new(
            Opcode::from(BuiltinOpcode::SH),
            2,
            1,
            0,
            InstructionType::SType,
        );
        let instruction = ShInstruction::decode(&bare_instruction, &cpu.registers);

        let result = instruction.memory_write(&mut memory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VMError::UnalignedMemoryWrite(0x1001)
        ));
    }

    #[test]
    fn test_sh_overflow() {
        let mut cpu = Cpu::default();
        let mut memory = setup_memory();

        cpu.registers.write(Register::X1, u32::MAX);
        cpu.registers.write(Register::X2, 0xABCD);

        let bare_instruction = Instruction::new(
            Opcode::from(BuiltinOpcode::SH),
            2,
            1,
            1,
            InstructionType::SType,
        );
        let instruction = ShInstruction::decode(&bare_instruction, &cpu.registers);

        let result = instruction.memory_write(&mut memory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VMError::AddressCalculationOverflow
        ));
    }

    // TODO: depending on the memory model, we need to test out of bound memory access
}
