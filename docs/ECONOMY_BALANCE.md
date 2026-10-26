# Economy pacing model

Reproduce: node scripts/economy-report.js. Model assumes 65% accuracy, the standard 55/20/20/5 enemy mix, all cash collected, and continuous ammo availability at the terminal. It excludes travel time and damage taken; this is a resource model, not a playtest. Repair cost is amortized at $0.50 per shot (5 condition per 100 shots, $500 per 50 condition). Prices and damage are unchanged.

Average kill payout: $30.5.

| Weapon | Shots/kill incl. misses | Ammo + repair/kill | Net cash/kill |
| --- | --- | --- | --- |
| Pistol | 9.2 | $20.0 | $10.5 |
| Helsing | 13.8 | $29.8 | $0.7 |
| Tikhar | 7.8 | $17.0 | $13.5 |
| Shambler | 7.9 | $17.2 | $13.3 |
| Shotgun | 5.8 | $12.7 | $17.8 |
| Kalash | 10.8 | $23.3 | $7.2 |
| VSV | 6.2 | $13.5 | $17.0 |
| Scorpion | 12.2 | $26.3 | $4.2 |
| Vector | 12.2 | $26.3 | $4.2 |
| G36 | 9.2 | $20.0 | $10.5 |

Wave-clear bonus is $75 + $25 × wave number; each fifth wave pays $500 instead. Optional event rewards rotate $150/$250/$350. Waves1–5 only contain zombies/runners with normalized55:20 weights: average payout $23.33. First-wave gross expected income is30 × $23.33 + $100 + $150 = $950. Pistol ammo/wear costs about $520 at this accuracy; one small medkit costs $100, leaving roughly $330. Eyes unlock on wave6; brutes unlock on wave11; the table above uses the full late-game mix. Buying the first $2,000 primary therefore takes several completed waves; $6,000 guns are longer-term goals. Missing events or taking more damage delays purchases. Reaching the terminal during assaults remains necessary: starting 92 rounds cannot clear 30 enemies. No automatic ammo or health gifts are introduced.
