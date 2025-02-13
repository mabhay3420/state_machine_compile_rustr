#![allow(unused)]
use std::collections::{HashMap, HashSet};

use crate::parser::{Condition, ParseTree, Transition, TransitionStep};
use inkwell::basic_block::BasicBlock;
use inkwell::context::Context;
use inkwell::targets::TargetTriple;
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

// Trait for converting to LLVM IR
pub trait ToLlvmIr {
    fn to_llvm_ir(&self) -> String;
}

// TODO - Write this to minimise the work for printing something
// fn print_call(builder: inkwell::builder::Builder, prompt: &str, name: &str, printf_fn: inkwell::values::FunctionValue, print_args: &[inkwell::values::BasicValueEnum], print_call_name: &str) {
//     let print_steps_format = builder.build_global_string_ptr(prompt, name).unwrap();
//     let mut print_all_args = [print_steps_format.as_basic_value_enum()];
//     print_all_args.iter().chain(print_args);
//     builder.build_call(printf_fn, &print_all_args,print_call_name);
// }

impl ToLlvmIr for ParseTree {
    fn to_llvm_ir(&self) -> String {
        // Create LLVM context, module, and builder
        let context = Context::create();
        let module = context.create_module("tape_machine_fixed");
        let builder = context.create_builder();

        // Set target triple for macOS on ARM64
        let triple = TargetTriple::create("arm64-apple-macosx13.0.0");
        module.set_triple(&triple);

        // Define basic LLVM types
        let i32_type = context.i32_type();
        let i8_type = context.i8_type();
        let ptr_type = context.ptr_type(AddressSpace::default());

        // Declare external functions (printf, malloc, scanf)
        let printf_type = i32_type.fn_type(&[ptr_type.into()], true);
        let printf_fn = module.add_function("printf", printf_type, None);

        let malloc_type = ptr_type.fn_type(&[i32_type.into()], false);
        let malloc_fn = module.add_function("malloc", malloc_type, None);

        let scanf_type = i32_type.fn_type(&[ptr_type.into()], true);
        let scanf_fn = module.add_function("scanf", scanf_type, None);

        // Define main function
        let main_type = i32_type.fn_type(&[], false);
        let main_fn = module.add_function("main", main_type, None);

        // Actual instruction building starts from here
        let entry = context.append_basic_block(main_fn, "entry");
        builder.position_at_end(entry);

        // Allocate and initialize variables
        let num_steps_ptr = builder.build_alloca(i32_type, "num_steps_ptr").unwrap();
        let arr_size_ptr = builder.build_alloca(i32_type, "arr_size_ptr").unwrap();
        let i32_0 = i32_type.const_int(0, false);

        // Prompt user for input (number of steps)
        let num_steps_prompt = builder
            .build_global_string_ptr("Enter number of steps: ", "num_steps_prompt")
            .unwrap();
        let scanf_format = builder
            .build_global_string_ptr("%d", "scanf_format")
            .unwrap();
        builder.build_call(
            printf_fn,
            &[num_steps_prompt.as_pointer_value().into()],
            "printf_call_1",
        );
        builder.build_call(
            scanf_fn,
            &[scanf_format.as_pointer_value().into(), num_steps_ptr.into()],
            "scanf_call_1",
        );

        // Load num_steps value
        let num_steps = builder
            .build_load(i32_type, num_steps_ptr, "num_steps")
            .unwrap()
            .into_int_value();
        // let num_steps = builder.build_load(i32_type, num_steps_ptr.as_basic_value_enum().into_pointer_value(), "num_steps").into_int_value();

        // Prompt user for input (array size)
        let arr_size_prompt = builder
            .build_global_string_ptr("Enter array size: ", "arr_size_prompt")
            .unwrap();
        builder.build_call(
            printf_fn,
            &[arr_size_prompt.as_pointer_value().into()],
            "printf_call_2",
        );
        builder.build_call(
            scanf_fn,
            &[scanf_format.as_pointer_value().into(), arr_size_ptr.into()],
            "scanf_call_2",
        );

        // Allocate tape dynamically using malloc
        let arr_size = builder
            .build_load(i32_type, arr_size_ptr, "arr_size")
            .unwrap()
            .into_int_value();
        let tape_ptr = builder
            .build_call(malloc_fn, &[arr_size.into()], "tape_array_malloc_call")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Initialize tape with 'X'

        let steps_loop = context.append_basic_block(main_fn, "steps_loop");
        let steps_loop_body = context.append_basic_block(main_fn, "steps_loop_body");
        let steps_loop_end = context.append_basic_block(main_fn, "steps_loop_end");
        let main_return = context.append_basic_block(main_fn, "main_return");

        // Initialize loop counter
        let current_tape_index_ptr = builder
            .build_alloca(i32_type, "current_tape_index_ptr")
            .unwrap();
        let current_step_ptr = builder.build_alloca(i32_type, "current_step_ptr").unwrap();
        let current_symbol_index_ptr = builder
            .build_alloca(i32_type, "current_symbol_index_ptr")
            .unwrap();
        let current_state_index_ptr = builder
            .build_alloca(i32_type, "current_state_index_ptr")
            .unwrap();
        builder.build_store(current_tape_index_ptr, i32_0);
        builder.build_store(current_step_ptr, i32_0);

        // TODO - Update it based on what the initial symbol is
        builder.build_store(current_symbol_index_ptr, i32_0);
        builder.build_store(current_state_index_ptr, i32_0);

        let print_steps_format = builder
            .build_global_string_ptr("All Symbols: %s\n", "print_current_step_format")
            .unwrap();

        let symbol_index_value_mapping = self
            .symbols
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}:{}", i, s))
            .collect::<Vec<String>>()
            .join(", ");

        let symbol_index_value_mapping_ptr = builder
            .build_global_string_ptr(&symbol_index_value_mapping, "symbol_index_value_mapping")
            .unwrap();
        builder.build_call(
            printf_fn,
            &[
                print_steps_format.as_pointer_value().into(),
                symbol_index_value_mapping_ptr.as_pointer_value().into(),
            ],
            "print_all_symbols",
        );

        let print_steps_format = builder
            .build_global_string_ptr("All States: %s\n", "print_current_step_format")
            .unwrap();

        let state_index_value_mapping = self
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}:{}", i, s))
            .collect::<Vec<String>>()
            .join(", ");

        let state_index_value_mapping_ptr = builder
            .build_global_string_ptr(&state_index_value_mapping, "state_index_value_mapping")
            .unwrap();
        builder.build_call(
            printf_fn,
            &[
                print_steps_format.as_pointer_value().into(),
                state_index_value_mapping_ptr.as_pointer_value().into(),
            ],
            "print_all_states",
        );
        builder.build_unconditional_branch(steps_loop);

        // Loop condition
        builder.position_at_end(steps_loop);
        let current_step_val = builder
            .build_load(i32_type, current_step_ptr, "current_step_val")
            .unwrap()
            .into_int_value();
        let current_symbol_index = builder
            .build_load(
                i32_type,
                current_symbol_index_ptr,
                "current_symbol_index_val",
            )
            .unwrap()
            .into_int_value();
        let current_state_index = builder
            .build_load(i32_type, current_state_index_ptr, "current_state_index_val")
            .unwrap()
            .into_int_value();
        // current_symbol_index * total_states + current_state_index
        let total_symbols = self.symbols.len();
        let total_states = self.states.len();
        let lhs = builder
            .build_int_mul(
                current_symbol_index,
                i32_type.const_int(total_states.try_into().unwrap(), false),
                "current_symbol_index__x__total_states",
            )
            .unwrap();
        let current_switch_case_number = builder
            .build_int_add(lhs, current_state_index, "current_switch_case_number")
            .unwrap();
        let step_limit_cond = builder
            .build_int_compare(
                IntPredicate::ULT,
                current_step_val,
                num_steps,
                "step_limit_cond",
            )
            .unwrap();
        builder.build_conditional_branch(step_limit_cond, steps_loop_body, steps_loop_end);

        // // Loop body: initialize tape with 'X'
        builder.position_at_end(steps_loop_body);
        let print_steps_format = builder
            .build_global_string_ptr("Current step: %d\n", "print_current_step_format")
            .unwrap();
        builder.build_call(
            printf_fn,
            &[
                print_steps_format.as_pointer_value().into(),
                current_step_val.into(),
            ],
            "current_step_print_call",
        );

        // Build a switch statement based on current step value
        // let current_step_nature = builder
        //     .build_int_signed_rem(
        //         current_step_val,
        //         i32_type.const_int(4, false),
        //         "is_even_check",
        //     )
        //     .unwrap();
        // Later we will use the current state and current symbol value
        let switch_default = context.append_basic_block(main_fn, "switch_default");
        let after_switch = context.append_basic_block(main_fn, "after_switch");
        let mut case_switch_mapping = vec![];
        let print_steps_format = builder
            .build_global_string_ptr("Symbol: %s State: %s\n", "print_current_step_format")
            .unwrap();

        let mut symbol_global_value_map: Vec<inkwell::values::GlobalValue> =
            Vec::with_capacity(total_symbols);
        // Can be improved as per clippy - but ignore
        for sym_index in 0..total_symbols {
            let symbol = builder
                .build_global_string_ptr(
                    &self.symbols[sym_index],
                    &format!("symbol_{}", self.symbols[sym_index]),
                )
                .unwrap();
            // symbol_global_value_map[sym_index] = symbol;
            symbol_global_value_map.push(symbol);
        }

        let mut state_global_value_map: Vec<inkwell::values::GlobalValue> =
            Vec::with_capacity(total_states);
        // Can be improved as per clippy - but ignore
        for state_index in 0..total_states {
            let state = builder
                .build_global_string_ptr(
                    &self.states[state_index],
                    &format!("state_{}", self.states[state_index]),
                )
                .unwrap();
            // state_global_value_map[state_index] = state;
            state_global_value_map.push(state);
        }

        // We will cover all cases for all combination of symbols and states
        // Total symbols = 5
        // Total states = 6
        // (sym_0, state_0): 0, (sym_0, state_1): 1, .... (sym_0, state_5): 5 : 1st row
        // (sym_1, state_0): 6, (sym_1, state_1): 7, ..... - 2nd row
        // ........
        // ...................  (sym_4, state_5): 29(4*6 + 5) - last row
        // General formula for case number = sym_index * total_states + state_index
        for switch_case_number in 0..total_symbols * total_states {
            // let switch_case_number = sym_index * total_states + state_index;
            let sym_index = switch_case_number / total_states;
            let state_index = switch_case_number % total_states;
            let symbol = symbol_global_value_map[sym_index];
            let state = state_global_value_map[state_index];
            let switch_case = context.append_basic_block(
                main_fn,
                &format!(
                    "state_{}_sym_{}",
                    self.states[state_index], self.symbols[sym_index]
                ),
            );
            builder.position_at_end(switch_case);
            builder.build_call(
                printf_fn,
                &[
                    print_steps_format.as_pointer_value().into(),
                    symbol.as_pointer_value().into(),
                    state.as_pointer_value().into(),
                ],
                "current_step_print_call",
            );
            builder.build_unconditional_branch(after_switch);
            case_switch_mapping.push((
                i32_type.const_int(switch_case_number.try_into().unwrap(), false),
                switch_case,
            ));
        }

        // Updates to state and symbol index based on conditions
        let state_to_index_map: HashMap<String, usize> = self
            .states
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i))
            .collect();
        let symbol_to_index_map: HashMap<String, usize> = self
            .symbols
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i))
            .collect();

        // For each initial state we will first collect
        // [{condition, steps, final_state}] vector map
        // Then we will process this to generate
        // [{switch_case_number,steps, final_state}] vector map

        let mut state_transition_map: HashMap<String, Vec<Transition>> = HashMap::new();
        for transition in self.transitions.iter() {
            let initial_state = &transition.initial_state;
            let condition = &transition.condition;
            let steps = &transition.steps;
            let final_state = &transition.final_state;
            state_transition_map
                .entry(initial_state.clone())
                .or_default()
                .push(transition.clone());
        }

        // initial_state -> [{symbols, steps, final_state}]
        let mut processed_state_transition_map: HashMap<
            String,
            Vec<(Vec<String>, Vec<TransitionStep>, String)>,
        > = HashMap::new();

        for (initial_state, transitions) in state_transition_map.iter_mut() {
            let mut inserted_symbols = HashSet::new();
            // Sort the transitions based on the condition so that we always process
            // OR condition first
            transitions.sort_by(|a, b| match (&a.condition, &b.condition) {
                (Condition::Star, Condition::OR(_)) => std::cmp::Ordering::Greater,
                (Condition::OR(_), Condition::Star) => std::cmp::Ordering::Less,
                _ => std::cmp::Ordering::Equal,
            });
            for transition in transitions.iter() {
                let final_state = transition.final_state.clone();
                let steps = transition.steps.clone();
                let mut symbols: Vec<String> = vec![];
                match &transition.condition {
                    Condition::OR(s) => {
                        symbols = s.to_vec();
                        inserted_symbols.extend(s);
                    }
                    Condition::Star => {
                        // symbols = self.symbols.iter().cloned().collect();
                        // Insert only those symbols which are not already inserted
                        symbols = self
                            .symbols
                            .iter()
                            .filter(|s| !inserted_symbols.contains(*s))
                            .cloned()
                            .collect();
                    }
                }
                // processed_state_transition_map
                //     .insert(initial_state.clone(), (symbols, steps, final_state.clone()));
                processed_state_transition_map
                    .entry(initial_state.clone())
                    .or_default()
                    .push((symbols, steps, final_state.clone()));
            }
        }

        for (initial_state, transitions) in processed_state_transition_map.iter() {
            for (matching_symbols, steps, final_state) in transitions {
                // Switch case numbers for this transition
                let switch_case_numbers = matching_symbols
                    .iter()
                    .map(|s| {
                        let symbol_index = symbol_to_index_map[s];
                        let state_index = state_to_index_map[initial_state];
                        symbol_index * total_states + state_index
                    })
                    .collect::<Vec<usize>>();

                for switch_case_number in switch_case_numbers {
                    let (_, switch_case) = case_switch_mapping[switch_case_number];
                    // builder.position_at_end(switch_case);

                    // Position at the instruction just before the last instruction
                    let unconditional_jump = switch_case.get_terminator().unwrap();
                    builder.position_before(&unconditional_jump);
                    for step in steps {
                        match step {
                            TransitionStep::L => {
                                let mut current_tape_index_value = builder
                                    .build_load(
                                        i32_type,
                                        current_tape_index_ptr,
                                        "current_tape_index_val",
                                    )
                                    .unwrap()
                                    .into_int_value();
                                current_tape_index_value = builder
                                    .build_int_sub(
                                        current_tape_index_value,
                                        i32_type.const_int(1, false),
                                        "move_left",
                                    )
                                    .unwrap();
                                builder
                                    .build_store(current_tape_index_ptr, current_tape_index_value);
                                // Move left
                            }
                            TransitionStep::R => {
                                // Move right
                                let mut current_tape_index_value = builder
                                    .build_load(
                                        i32_type,
                                        current_tape_index_ptr,
                                        "current_tape_index_val",
                                    )
                                    .unwrap()
                                    .into_int_value();
                                current_tape_index_value = builder
                                    .build_int_add(
                                        current_tape_index_value,
                                        i32_type.const_int(1, false),
                                        "move_right",
                                    )
                                    .unwrap();
                                builder
                                    .build_store(current_tape_index_ptr, current_tape_index_value);
                            }
                            TransitionStep::P(symbol) => {
                                // TODO - Print symbol index in the current index
                            }
                            TransitionStep::X => {
                                // Do nothing
                            }
                        }
                    }
                }
            }
        }

        builder.position_at_end(switch_default);
        let print_steps_format = builder
            .build_global_string_ptr("Default Remainder: %d\n", "print_current_step_format")
            .unwrap();
        builder.build_call(
            printf_fn,
            &[
                print_steps_format.as_pointer_value().into(),
                current_step_val.into(),
            ],
            "current_step_print_call",
        );
        builder.build_unconditional_branch(after_switch);

        // builder.position_at_end(switch_even_case);
        // let print_steps_format = builder
        //     .build_global_string_ptr("Remainder 0: %d\n", "print_current_step_format")
        //     .unwrap();
        // builder.build_call(
        //     printf_fn,
        //     &[
        //         print_steps_format.as_pointer_value().into(),
        //         current_step_val.into(),
        //     ],
        //     "current_step_print_call",
        // );
        // builder.build_unconditional_branch(after_switch);

        // builder.position_at_end(switch_odd_case);
        // let print_steps_format = builder
        //     .build_global_string_ptr("Remainder 1: %d\n", "print_current_step_format")
        //     .unwrap();
        // builder.build_call(
        //     printf_fn,
        //     &[
        //         print_steps_format.as_pointer_value().into(),
        //         current_step_val.into(),
        //     ],
        //     "current_step_print_call",
        // );
        // builder.build_unconditional_branch(after_switch);

        // Insert swicht statement in steps loop body
        builder.position_at_end(steps_loop_body);
        // print_call(builder, "Switch Default Case", "switch_default_print", printf_fn, , print_call_name);
        builder.build_switch(
            current_switch_case_number,
            switch_default,
            &case_switch_mapping,
        );

        builder.position_at_end(after_switch);
        let updated_current_step_val = builder
            .build_int_add(
                current_step_val,
                i32_type.const_int(1, false),
                "current_step_increment",
            )
            .unwrap();
        builder.build_store(current_step_ptr, updated_current_step_val);

        // TODO - Move this to every switch statement block
        // This will be updated based on the associated conditions
        let mut updated_current_symbol_index = builder
            .build_int_add(
                current_symbol_index,
                i32_type.const_int(2, false),
                "current_symbol_index_increment_1",
            )
            .unwrap();

        // TODO - This will be replaced by forcing the flow to go to
        // Invalid block if the current symbol doesn't exist in the
        // available options
        updated_current_symbol_index = builder
            .build_int_signed_rem(
                updated_current_symbol_index,
                i32_type.const_int(total_symbols.try_into().unwrap(), false),
                "clip_symbol_index",
            )
            .unwrap();

        builder.build_store(current_symbol_index_ptr, updated_current_symbol_index);

        // We will similarly update the state index later
        builder.build_unconditional_branch(steps_loop);

        // Loop end
        builder.position_at_end(steps_loop_end);
        let print_loop_end_format = builder
            .build_global_string_ptr("Reached end of steps loop.", "step_loop_end_format")
            .unwrap();
        builder.build_call(
            printf_fn,
            &[print_loop_end_format.as_pointer_value().into()],
            "step_loop_end_print",
        );
        builder.build_unconditional_branch(main_return);

        builder.position_at_end(main_return);
        builder.build_return(Some(&i32_type.const_int(0, false)));

        // Generate LLVM IR as a string
        module.print_to_string().to_string()
    }
}
