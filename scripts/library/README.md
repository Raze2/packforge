# Library generators

Some component files share a layout and are generated so they stay consistent:

| Script | Writes |
|---|---|
| `slotted.py` | `library/fefco/0200`, `0202`, `0203`, `0204` (`0201` is hand-written) |
| `others.py` | `library/sleeves/sleeve`, `library/trays/tray-glued-corners`, `library/cartons/tuck-end-*` |

```bash
python3 scripts/library/slotted.py library/fefco
(cd library && python3 ../scripts/library/others.py .)
UPDATE_GOLDEN=1 cargo test -p pf-core --test golden   # then review the golden diff
cargo test -p pf-core --test library
```

Edit the generator rather than the generated file, then regenerate.
