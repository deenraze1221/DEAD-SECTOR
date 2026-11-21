# Economy pacing model

Reproduce: node scripts/economy-report.js. Model assumes 65% accuracy, the standard 55/20/20/5 enemy mix, all cash collected, and continuous ammo availability at the terminal. It excludes travel time and damage taken; this is a resource model, not a playtest. Repair cost is amortized at $0.50 per shot (5 condition per 100 shots, $500 per 50 condition). Prices, magazines and damage follow the current weapon configuration.

Average kill payout: $28.5.

| Weapon | Shots/kill incl. misses | Ammo + repair/kill | Net cash/kill |
| --- | --- | --- | --- |
| Pistol | 8.9 | $13.4 | $15.1 |
| USP | 7.5 | $11.3 | $17.2 |
| Blackthorn | 4.8 | $7.2 | $21.3 |
| Ironhawk | 3.2 | $4.7 | $23.8 |
| Kureyka | 4.4 | $6.6 | $21.9 |
| Kemchug | 19.3 | $29.0 | $-0.5 |
| Biryusa | 12.1 | $18.1 | $10.4 |
| Taiga | 4.4 | $6.6 | $21.9 |
| Bereg | 10.7 | $16.0 | $12.5 |
| Angara | 5.8 | $8.7 | $19.8 |
| Sisim | 11.7 | $17.5 | $11.0 |
| Chulym | 8.9 | $13.4 | $15.1 |
| Sayan | 7.4 | $11.1 | $17.4 |
| Vetluga | 7.5 | $11.3 | $17.2 |
| Kazyr | 5.7 | $8.5 | $20.0 |
| Katanga | 8.9 | $13.4 | $15.1 |

Wave-clear bonus is $75 + $25 × wave number; each fifth wave pays $500 instead. Shared missions roll with15% chance from wave3 and pay300–600 per player. Early-wave zombies/runners average $26 per kill. First-wave expected kill income plus clear bonus is15 × $26 + $100 ≈ $490, before ammunition/repair and healing costs. New ammunition packs contain50 rounds for $50. The first two waves have no missions. No automatic ammo or health gifts are introduced.
