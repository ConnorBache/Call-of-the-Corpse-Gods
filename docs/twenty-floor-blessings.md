# Twenty-floor blessing changes

Only runs marked `compressedDepths` use these rules. Legacy Normal/Fast saves retain their original reward and level costs.

- Heart blessing rewards are half the former Fast reward (authored base exp ×1 rather than ×2).
- Heart permanent stat rewards retain the former Fast ×2 multiplier, with existing stat rounding unchanged.
- Level 2 costs 1 blessing, level 3 costs 2, level 4 costs 4, and each subsequent level doubles the cost.
- Authored fractional blessing rewards retain their exact value; no rounding is added. Existing score completeness rules for fractional authored rewards remain unchanged.
- Veridian Sap still fills the deficit toward the next level and now uses the actual next-level threshold. It was never doubled by Fast Mode, so no additional halving is applied.
- Heart reward data remains unmodified on carried items; previews, consumption, and debug accrual derive effective rewards through the same helper.
