# Economy pacing model

Reproduce: node scripts/economy-report.js. Model assumes 65% accuracy, the standard 55/20/20/5 enemy mix, all cash collected, and continuous ammo availability at the terminal. It excludes travel time and damage taken; this is a resource model, not a playtest. Repair cost is amortized at $0.50 per shot (5 condition per 100 shots, $500 per 50 condition). Prices, magazines and damage follow the current weapon configuration.

Average kill payout: $32.5.

| Weapon | Shots/kill incl. misses | Ammo + repair/kill | Net cash/kill |
| --- | --- | --- | --- |
| Pistol | 9.2 | $13.8 | $18.7 |
| Helsing | 12.2 | $18.2 | $14.3 |
| Tikhar | 3.3 | $5.0 | $27.5 |
| Shambler | 6.3 | $9.5 | $23.0 |
| Shotgun | 4.9 | $7.4 | $25.1 |
| Kalash | 7.9 | $11.9 | $20.6 |
| VSV | 5.8 | $8.8 | $23.7 |
| Scorpion | 9.2 | $13.8 | $18.7 |
| Vector | 10.8 | $16.2 | $16.3 |
| G36 | 10.8 | $16.2 | $16.3 |

Wave-clear bonus is $75 + $25 × wave number; each fifth wave pays $500 instead. Shared missions roll with15% chance from wave3 and pay300–600 per player. Early-wave zombies/runners average $26 per kill. First-wave expected kill income plus clear bonus is15 × $26 + $100 ≈ $490, before ammunition/repair and healing costs. New ammunition packs contain50 rounds for $50. The first two waves have no missions. No automatic ammo or health gifts are introduced.
