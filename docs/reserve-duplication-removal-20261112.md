# Duplicate reserve value removal, 2026-11-12

The designer answered "Yeah" to removing the second reserve energy value in ability details, while leaving its cost-line value.

On older R51 branch base d81de6e43e499dd3ec1ff250589a937925d1ae00, ability_ui.rs first adds the reserve summary to cost. It later repeated that summary at the start of the description. Only the second `{summary}. ` display prefix was removed. The cost line and the complete existing payment/refill rules remain unchanged, including this branch's fixed refill value of 50. The newline and See The Future exception text remain. The second unused binding is now Some(_) and the no-longer-needed format! wrapper was removed; presence condition unchanged.

Verified the two assembled reserve uses before versus one afterward, exact narrow diff, and Rust syntax parsing. No JSON changed. No Cargo/runtime/visual tests run. This is not the missing latest integrated R52 source. Main is not merged or modified.
