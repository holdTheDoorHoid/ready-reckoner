# Price observations

The retail observations behind every price band in `content/items/*.toml`. Each priced item has at
least two observations; its band runs from the lowest to the highest unit price here (the
`rr-content` test `price_bands_match_the_observation_log` checks this). Cited in the app as
`rr_price_observations_2026_09`.

- **Observed 2026-09-25** unless the source says otherwise (a few rows reuse observations from
  `docs/research/supply-standards.md` made the same day, and two use archived store pages).
- **Sources** are the stores and agencies that served prices to an automated request: open product
  feeds of specialty retailers, a restaurant and janitorial supplier, and federal price series (BLS
  average prices, USDA food plans) and survey averages (KFF copays). Big-box stores blocked automated
  requests, so everyday store-brand prices are under-represented; several bands are therefore on the
  high side, and each item's note says what its band reflects.
- **Unit price** = listed price ÷ units, in the item's own unit. Composite rows (two listings bought
  together) and conversions (per 2,000 kcal, per person-month, per cycle) say so in the listing text.
- Store and product names appear here for auditing only; the app never shows brands or store links
  (`docs/PRINCIPLES.md` §8). Prices drift: this log is refreshed with the data packs, and the user's
  own recorded prices always replace these bands.

| Item | Source | Listing | Listed price (USD) | Units | Unit price (USD) | Per | URL |
|---|---|---|---|---|---|---|---|
| `water_stored_bottled` | webstaurantstore.com | Purified bottled water, 24 x 16.9 fl oz (3.17 gal) | 3.59 | 3.17 | 1.13 | gallon | https://www.webstaurantstore.com/16-9-oz-purified-bottled-water-case/103WATERBOTL.html |
| `water_stored_bottled` | webstaurantstore.com | Spring water, 6 x 1 gallon | 5.29 | 6 | 0.88 | gallon | https://www.webstaurantstore.com/crystal-geyser-1-gallon-natural-spring-water-case/103WATER61.html |
| `water_jug_7gal` | relianceoutdoors.com | 7-gallon (26 L) rigid water container with spigot | 23.99 | 1 | 23.99 | jug | https://relianceoutdoors.com/products/aqua-tainer-4g-15l |
| `water_jug_7gal` | relianceoutdoors.com | 7-gallon (26 L) rigid water container, second model | 20.99 | 1 | 20.99 | jug | https://relianceoutdoors.com/products/jumbo-tainer-7g-26l |
| `water_drum_55gal` | augasonfarms.com | 55-gallon water storage barrel kit with pump and treatment (sale; list $309.99; local pickup) | 129.99 | 1 | 129.99 | drum | https://www.augasonfarms.com/products/water-treatment-storage-kit |
| `water_drum_55gal` | thereadystore.com | 55-gallon water storage barrel (sale; list $249.99) | 199.99 | 1 | 199.99 | drum | https://thereadystore.com/products/55-gallon-water-barrel-1 |
| `water_bleach_unscented` | webstaurantstore.com | Regular bleach, 128 fl oz, case of 6 | 39.49 | 6 | 6.58 | bottle | https://www.webstaurantstore.com/pure-bright-1-gallon-128-oz-bleach-case/147BL6.html |
| `water_bleach_unscented` | webstaurantstore.com | Concentrated disinfecting bleach, 81 fl oz, case of 6 | 52.49 | 6 | 8.75 | bottle | https://www.webstaurantstore.com/cloroxpro-clorox-clo32263-81-oz-concentrated-disinfecting-bleach-with-cloromax-case/19B32263.html |
| `water_filter_gravity` | lifestraw.com | Family gravity water filter (bacteria and parasites) | 54.95 | 1 | 54.95 | filter | https://lifestraw.com/products/lifestraw-family-emergency-water-filter |
| `water_filter_gravity` | lifestraw.com | 8-liter gravity filter system | 95.95 | 1 | 95.95 | filter | https://lifestraw.com/products/lifestraw-peak-series-gravity-filter-system-8l |
| `water_personal_filter` | lifestraw.com | Personal straw filter | 17.95 | 1 | 17.95 | filter | https://lifestraw.com/products/lifestraw |
| `water_personal_filter` | beprepared.com | Survival straw filter | 24.95 | 1 | 24.95 | filter | https://beprepared.com/products/aquamira-survival-straw |
| `water_personal_filter` | 4patriots.com | Personal water filter straw, 1-pack | 29.00 | 1 | 29.00 | filter | https://4patriots.com/products/patriot-pure-personal-water-filter-straw |
| `water_chlorine_dioxide` | mypatriotsupply.com | Chlorine dioxide water treatment, two-part drops (1 pack) | 9.99 | 1 | 9.99 | pack | https://mypatriotsupply.com/products/chlorine-dioxide-water-treatment |
| `water_chlorine_dioxide` | beprepared.com | Chlorine dioxide water treatment, two-part drops | 16.99 | 1 | 16.99 | pack | https://beprepared.com/products/aquamira-chlorine-dioxide-water-treatment |
| `food_pantry_rotation` | fns.usda.gov | Thrifty Food Plan, reference family of four, August 2026: $236.30 a week = 28 person-days (about 2,000 kcal each) | 236.30 | 28 | 8.44 | 2,000 kcal | https://www.fns.usda.gov/sites/default/files/resource-files/CostofFoodAug2026Thrifty.pdf |
| `food_pantry_rotation` | fns.usda.gov | Low-Cost Food Plan, male 19-50, August 2026: $73.20 a week | 73.20 | 7 | 10.46 | 2,000 kcal | https://www.fns.usda.gov/sites/default/files/resource-files/CostofFoodAug2026LowModLib.pdf |
| `food_bulk_staples` | bls.gov | White long-grain rice, US city average, August 2026: $1.111 per lb; 1,656 kcal per lb (FoodData Central 169756) | 1.11 | 0.8282 | 1.34 | 2,000 kcal | https://data.bls.gov/timeseries/APU0000701312 |
| `food_bulk_staples` | bls.gov | Dried beans, US city average, August 2026: $1.630 per lb; 1,574 kcal per lb (FoodData Central 175199) | 1.63 | 0.787 | 2.07 | 2,000 kcal | https://data.bls.gov/timeseries/APU0000714233 |
| `food_bulk_staples` | churchofjesuschrist.org | Home Storage Center hard red wheat, 5.5-lb can, January 2026: 8,158 kcal (FoodData Central 168890) | 7.00 | 4.079 | 1.72 | 2,000 kcal | https://assets.churchofjesuschrist.org/01/b4/01b41d0ffdbf11ebbb5eeeeeac1e9b2afdc29be5/food_storage_center_products.pdf |
| `food_bulk_staples` | churchofjesuschrist.org | Home Storage Center pinto beans, 5.2-lb can, January 2026: 8,184 kcal | 8.83 | 4.092 | 2.16 | 2,000 kcal | https://assets.churchofjesuschrist.org/01/b4/01b41d0ffdbf11ebbb5eeeeeac1e9b2afdc29be5/food_storage_center_products.pdf |
| `food_freeze_dried` | mypatriotsupply.com | 4-week kit sold as 2,000+ kcal a day (56,000 kcal) | 277.95 | 28 | 9.93 | 2,000 kcal | https://mypatriotsupply.com/products/4-week-emergency-food-supply-2-000-calories-day |
| `food_freeze_dried` | mountainhouse.com | 30-day kit, about 1,720 kcal a day (51,600 kcal; research supply §2.7) | 939.99 | 25.8 | 36.43 | 2,000 kcal | https://mountainhouse.com/products/30-day-emergency-food-supply-kit |
| `food_manual_can_opener` | webstaurantstore.com | Manual can opener | 6.69 | 1 | 6.69 | opener | https://www.webstaurantstore.com/choice-co47bk-portable-can-opener-with-black-handle/407CO47BK.html |
| `food_manual_can_opener` | webstaurantstore.com | Large handheld crank can opener | 11.49 | 1 | 11.49 | opener | https://www.webstaurantstore.com/garde-cohh-lc-large-handheld-crank-can-opener/181COHHLC.html |
| `food_camp_stove` | gasone.com | One-burner butane stove (7,650 BTU) with griddle | 39.99 | 1 | 39.99 | stove | https://www.gasone.com/products/gs-1000g-kit |
| `food_camp_stove` | gasone.com | One-burner propane camp stove | 49.99 | 1 | 49.99 | stove | https://www.gasone.com/products/ps-1500mb |
| `food_cooking_fuel` | webstaurantstore.com | Butane fuel, 8 oz canisters, pack of 48 | 69.99 | 48 | 1.46 | 8-ounce canister | https://www.webstaurantstore.com/choice-butane-fuel-refill-8-oz-canister-pack/174BUTANE48KIT.html |
| `food_cooking_fuel` | webstaurantstore.com | Butane fuel, 8 oz canisters, case of 12 | 32.49 | 12 | 2.71 | 8-ounce canister | https://www.webstaurantstore.com/sterno-products-50162-butane-fuel-refill-8-oz-canister-case/22350162.html |
| `food_infant_formula` | us.kendamil.com | Infant formula powder, 28.2 oz can (cow's milk) | 33.99 | 28.2 | 1.21 | ounce of powder | https://us.kendamil.com/products/classic-first-infant-milk |
| `food_infant_formula` | us.kendamil.com | Infant formula powder, 28.2 oz can (goat's milk) | 44.99 | 28.2 | 1.60 | ounce of powder | https://us.kendamil.com/products/goat-first-infant-milk |
| `food_fridge_thermometers` | webstaurantstore.com | Dial refrigerator/freezer thermometer (two needed) | 2.39 | 0.5 | 4.78 | pair | https://www.webstaurantstore.com/choice-2-dial-refrigerator-freezer-thermometer/9142DRFT.html |
| `food_fridge_thermometers` | webstaurantstore.com | Digital refrigerator/freezer thermometer (two needed) | 5.29 | 0.5 | 10.58 | pair | https://www.webstaurantstore.com/avatemp-digital-refrigerator-freezer-thermometer/914FTTHERM.html |
| `food_cooler` | igloocoolers.com | 30-quart hard cooler | 30.99 | 1 | 30.99 | cooler | https://www.igloocoolers.com/products/profile-ii-30-qt-cooler |
| `food_cooler` | igloocoolers.com | 30-quart hard cooler, second model | 49.99 | 1 | 49.99 | cooler | https://www.igloocoolers.com/products/ecocool-30-qt-cooler |
| `med_reserve_supply` | kff.org | Average copay for a first-tier (generic) drug, employer plans, 2025: $12 per 30-day fill | 12.00 | 30 | 0.40 | day of one person's medicine | https://www.kff.org/health-costs/2025-employer-health-benefits-survey/ |
| `med_reserve_supply` | kff.org | Average copay for a second-tier (preferred brand) drug, employer plans, 2025: $40 per 30-day fill | 40.00 | 30 | 1.33 | day of one person's medicine | https://www.kff.org/health-costs/2025-employer-health-benefits-survey/ |
| `med_first_aid_kit` | redcross.org (store, archived 2026-08-30) | 299-piece all-purpose first aid kit | 34.95 | 1 | 34.95 | kit | https://web.archive.org/web/20260830185522/https://www.redcross.org/store/first-aid-supplies/first-aid-kits |
| `med_first_aid_kit` | redcross.org (store, archived 2026-08-30) | 115-piece deluxe family first aid kit | 49.95 | 1 | 49.95 | kit | https://web.archive.org/web/20260830185522/https://www.redcross.org/store/first-aid-supplies/first-aid-kits |
| `med_otc_basics` | webstaurantstore.com | Four boxed unit-dose medicines: store-brand pain reliever 100 ($8.79), anti-diarrhea 6 ($6.99), upset-stomach 24 ($6.19), allergy 50 ($11.99) | 33.96 | 1 | 33.96 | set | https://www.webstaurantstore.com/medi-first-ibuprofen-tablets-box/57780833.html |
| `med_otc_basics` | webstaurantstore.com | Same set with a name-brand pain reliever 100 ($26.49) | 51.66 | 1 | 51.66 | set | https://www.webstaurantstore.com/advil-15000-ibuprofen-tablets-box/86615000.html |
| `med_n95_respirators` | webstaurantstore.com | NIOSH N95 respirators, pack of 20 | 14.49 | 20 | 0.72 | respirator | https://www.webstaurantstore.com/3m-8200-n95-particulate-respirator-pack/3998200.html |
| `med_n95_respirators` | webstaurantstore.com | NIOSH N95 respirators, pack of 20, second model | 25.99 | 20 | 1.30 | respirator | https://www.webstaurantstore.com/3m-8210-n95-particulate-respirator-pack/3998210.html |
| `med_cooler_refrigerated_rx` | webstaurantstore.com | Small insulated soft cooler bag ($12.99) plus two 24 oz gel packs from a 24-pack ($20.99) | 14.74 | 1 | 14.74 | set | https://www.webstaurantstore.com/choice-insulated-leak-proof-cooler-bag-soft-cooler-bag-with-shoulder-strap-holds-24-cans/ |
| `med_cooler_refrigerated_rx` | igloocoolers.com + webstaurantstore.com | 12-can soft satchel cooler ($34.99) plus two gel packs ($0.87 each) | 36.73 | 1 | 36.73 | set | https://www.igloocoolers.com/products/tag-along-too-12-can-satchel-cooler |
| `med_bleeding_control_kit` | mymedic.com | Windlass tourniquet ($37.95) plus 6-inch emergency pressure bandage ($9.95) | 47.90 | 1 | 47.90 | kit | https://mymedic.com/products/sam-xt-tourniquet |
| `med_bleeding_control_kit` | mymedic.com | Packaged bleeding-control kit | 94.95 | 1 | 94.95 | kit | https://mymedic.com/products/med-pack-bleed-stopper-first-aid-trauma-hemorrhage |
| `security_motion_light` | mypatriotsupply.com | Solar-powered LED motion-sensor outdoor light | 24.49 | 1 | 24.49 | light | https://mypatriotsupply.com/products/ready-hour-outdoor-solar-powered-212-led-motion-sensor-light |
| `security_motion_light` | beprepared.com | Solar-powered LED motion-sensor outdoor light, second store | 24.99 | 1 | 24.99 | light | https://beprepared.com/products/outdoor-solar-powered-212-led-motion-sensor-light-by-ready-hour |
| `san_twin_bucket_toilet` | relianceoutdoors.com + webstaurantstore.com | 5-gallon bucket toilet with seat ($25.99) plus a second 5-gallon bucket with lid ($13.99) | 39.98 | 1 | 39.98 | kit | https://relianceoutdoors.com/products/luggable-loo-portable-toilet |
| `san_twin_bucket_toilet` | legacyfoodstorage.com | Complete bucket toilet set with waste gel | 52.99 | 1 | 52.99 | kit | https://legacyfoodstorage.com/products/complete-toilet-set-with-ecogel |
| `san_toilet_bags` | webstaurantstore.com | 13-gallon 0.9-mil drawstring bags, case of 200 (unscented) | 22.49 | 200 | 0.11 | bag | https://www.webstaurantstore.com/lavex-13-gallon-0-9-mil-24-x-27-unscented-low-density-medium-duty-white-tall-kitchen-drawstring-can-liner-trash-bag-case/501LD13WU.html |
| `san_toilet_bags` | webstaurantstore.com | 13-gallon 0.9-mil drawstring bags, case of 200 (odor-control) | 22.49 | 200 | 0.11 | bag | https://www.webstaurantstore.com/lavex-13-gallon-0-9-mil-24-x-27-odor-guard-low-density-medium-duty-white-tall-kitchen-drawstring-can-liner-trash-bag-case/501LD13WOG.html |
| `san_toilet_paper` | webstaurantstore.com | 2-ply 500-sheet standard rolls, case of 96 | 46.99 | 96 | 0.49 | roll | https://www.webstaurantstore.com/lavex-janitorial-individually-wrapped-2-ply-standard-500-sheet-toilet-paper-roll-case/5002TP3X496E.html |
| `san_toilet_paper` | webstaurantstore.com | 2-ply 451-sheet standard rolls, name brand, case of 60 | 66.99 | 60 | 1.12 | roll | https://www.webstaurantstore.com/cottonelle-professional-451-sheet-toilet-paper-roll-case/5002TP17713.html |
| `san_hand_sanitizer` | webstaurantstore.com | Alcohol gel hand sanitizer, 1 gallon, case of 4 (per 12 oz) | 89.99 | 42.67 | 2.11 | 12-ounce bottle | https://www.webstaurantstore.com/noble-chemical-1-gallon-alcohol-based-gel-instant-hand-sanitizer-with-pump-case/ |
| `san_hand_sanitizer` | webstaurantstore.com | Name-brand alcohol hand sanitizer, 12 oz bottles, case of 12 | 79.99 | 12 | 6.67 | 12-ounce bottle | https://www.webstaurantstore.com/purell-3659-12-advanced-12-oz-instant-hand-sanitizer-12-case/ |
| `san_soap` | webstaurantstore.com | Wrapped 0.8 oz bath bars (500/case, $72.49) for 250 g plus 50-lb laundry powder ($55.49) for 200 g: one person-month | 2.09 | 1 | 2.09 | person-month | https://www.webstaurantstore.com/novo-essentials-0-8-oz-hotel-and-motel-wrapped-bath-soap-bar-case/ |
| `san_soap` | drbronner.com + webstaurantstore.com | 5 oz castile bar ($5.19) for 250 g plus 18-lb laundry powder ($37.49) for 200 g: one person-month | 10.08 | 1 | 10.08 | person-month | https://www.drbronner.com/products/sandalwood-jasmine-pure-castile-bar-soap |
| `san_menstrual_products` | webstaurantstore.com | Pads, 44-count packs, case of 3 ($28.49): 20 pads for one cycle | 28.49 | 6.6 | 4.32 | cycle's supply | https://www.webstaurantstore.com/always-ultra-thin-44count-unscented-menstrual-pad-with-wings-size-1-regular-case/ |
| `san_menstrual_products` | webstaurantstore.com | Tampons, 18-count boxes, case of 12 ($79.99): 20 tampons for one cycle | 79.99 | 10.8 | 7.41 | cycle's supply | https://www.webstaurantstore.com/tampax-pearl-18-count-tampon-with-plastic-applicator-regular-case/ |
| `san_diapers` | webstaurantstore.com | Size 1 diapers, case of 108 | 34.99 | 108 | 0.32 | diaper | https://www.webstaurantstore.com/huggies-snug-dry-size-1-baby-diapers-case/50054645.html |
| `san_diapers` | webstaurantstore.com | Newborn diapers, case of 124 | 57.99 | 124 | 0.47 | diaper | https://www.webstaurantstore.com/pampers-swaddlers-newborn-size-baby-diapers-case/63282835.html |
| `thermal_sleeping_bag` | webstaurantstore.com | Heavy 80% wool blanket, 5 x 7 ft | 25.00 | 1 | 25.00 | person | https://www.webstaurantstore.com/kemp-usa-gray-5-x-7-fire-resistant-80-wool-blanket-10-606/896106 |
| `thermal_sleeping_bag` | tetonsports.com | 0°F sleeping bag, regular | 55.99 | 1 | 55.99 | person | https://www.tetonsports.com/products/fahrenheit-0f-sleeping-bag |
| `thermal_battery_fan` | webstaurantstore.com | Rechargeable handheld fan | 10.99 | 1 | 10.99 | fan | https://www.webstaurantstore.com/visionair-4-black-rechargeable-battery-operated-3-speed-usb-handheld-fan/ |
| `thermal_battery_fan` | o2cool.com | Battery desk and handheld fan | 22.99 | 1 | 22.99 | fan | https://www.o2cool.com/products/handy-fan |
| `thermal_indoor_thermometer` | webstaurantstore.com | Stick-on dial indoor/outdoor thermometer | 2.19 | 1 | 2.19 | thermometer | https://www.webstaurantstore.com/taylor-5380n-1-3-4-mini-window-stick-on-indoor-outdoor-thermometer/ |
| `thermal_indoor_thermometer` | webstaurantstore.com | 12-inch wall thermometer with humidity dial | 15.49 | 1 | 15.49 | thermometer | https://www.webstaurantstore.com/taylor-497j-12-dial-indoor-outdoor-wall-thermometer-with-hygrometer/ |
| `thermal_emergency_blankets` | webstaurantstore.com | Foil emergency blanket | 0.50 | 1 | 0.50 | blanket | https://www.webstaurantstore.com/kemp-usa-mylar-foil-emergency-thermal-blanket-10-601/89610601.html |
| `thermal_emergency_blankets` | beprepared.com | Emergency blanket | 4.95 | 1 | 4.95 | blanket | https://beprepared.com/products/z_legacy_z_cc_02_emergency-blanket |
| `power_headlamp` | mypatriotsupply.com | Rechargeable LED headlamp with motion sensor | 14.99 | 1 | 14.99 | headlamp | https://mypatriotsupply.com/products/rechargeable-sensor-headlamp |
| `power_headlamp` | webstaurantstore.com | Rechargeable LED headlamp, 400 lumens | 22.99 | 1 | 22.99 | headlamp | https://www.webstaurantstore.com/vision-ultra-hd-rechargeable-headlamp/199ENHDFRLP.html |
| `power_lantern` | 4patriots.com | Inflatable solar LED lantern and phone charger | 29.00 | 1 | 29.00 | lantern | https://4patriots.com/products/solantern-air-inflatable-solar-lantern-charger |
| `power_lantern` | beprepared.com | USB rechargeable LED lantern with 3,000 mAh power bank | 29.95 | 1 | 29.95 | lantern | https://beprepared.com/products/usb-emergency-lantern-from-ready-hour |
| `power_batteries` | webstaurantstore.com | AA alkaline batteries, pack of 24 (industrial) | 10.49 | 1 | 10.49 | 24-pack | https://www.webstaurantstore.com/rayovac-alaa-24ppj-ultra-pro-industrial-aa-alkaline-batteries-24-pack/ |
| `power_batteries` | webstaurantstore.com | AA alkaline batteries, pack of 24 (name brand) | 19.99 | 1 | 19.99 | 24-pack | https://www.webstaurantstore.com/energizer-max-e91bp-24-aa-alkaline-batteries-pack/199E91AAP24.html |
| `power_bank` | us.ecoflow.com | 20,000 mAh power bank, 45 W, built-in cable | 41.39 | 1 | 41.39 | power bank | https://us.ecoflow.com/products/ecoflow-rapid-power-bank-20k-45w-built-in-cable |
| `power_bank` | mypatriotsupply.com | 20,000 mAh power bank, 65 W | 59.99 | 1 | 59.99 | power bank | https://mypatriotsupply.com/products/65w-power-bank-by-grid-doctor |
| `power_station` | us.ecoflow.com | 1,024 Wh portable power station | 499.00 | 1 | 499.00 | power station | https://us.ecoflow.com/products/delta-3-classic-portable-power-station |
| `power_station` | jackery.com | 1,070 Wh LiFePO4 power station, 1,500 W (research supply §6.2) | 559.00 | 1 | 559.00 | power station | https://www.jackery.com/products/jackery-explorer-1000-v2 |
| `power_generator` | westinghouseoutdoorpower.com | 2,550 W inverter generator | 499.00 | 1 | 499.00 | generator | https://westinghouseoutdoorpower.com/products/igen2550-inverter-generator-no-led |
| `power_generator` | westinghouseoutdoorpower.com | 2,800 W inverter generator with CO sensor | 549.00 | 1 | 549.00 | generator | https://westinghouseoutdoorpower.com/products/igen2800c-inverter-generator-with-co-sensor |
| `power_generator` | powerequipment.honda.com (archived 2026-09-14) | 2,200 W inverter generator, list price | 1199.00 | 1 | 1199.00 | generator | https://web.archive.org/web/20260914102101/https://powerequipment.honda.com/generators/models/eu2200i |
| `power_generator_fuel` | bls.gov | Regular gasoline, US city average, August 2026 | 4.20 | 1 | 4.20 | gallon | https://data.bls.gov/timeseries/APU000074714 |
| `power_generator_fuel` | bls.gov | Regular gasoline, US city average, July 2026 | 4.09 | 1 | 4.09 | gallon | https://data.bls.gov/timeseries/APU000074714 |
| `power_solar_panel` | jackery.com | 100 W folding solar panel | 199.00 | 1 | 199.00 | panel | https://www.jackery.com/products/add-on-jackery-solarsaga-100-prime-solar-panel |
| `power_solar_panel` | mypatriotsupply.com | 100 W portable solar panel | 247.00 | 1 | 247.00 | panel | https://mypatriotsupply.com/products/solar-panel-100w-by-grid-doctor |
| `power_solar_panel` | jackery.com | 100 W lightweight folding solar panel | 249.00 | 1 | 249.00 | panel | https://www.jackery.com/products/jackery-solarsaga-100-air |
| `power_device_battery` | westinghouseoutdoorpower.com | 183 Wh portable power station | 169.00 | 1 | 169.00 | battery | https://westinghouseoutdoorpower.com/products/igen400s-portable-power-station |
| `power_device_battery` | us.ecoflow.com | 286 Wh portable power station (base unit) | 299.00 | 1 | 299.00 | battery | https://us.ecoflow.com/products/ecoflow-river-3-plus-portable-power-station-flash-sale |
| `comms_noaa_radio` | midlandusa.com (research supply §7, 2026-09-25) | Desktop NOAA Weather Radio with tone alert and battery backup | 54.99 | 1 | 54.99 | radio | https://midlandusa.com/products.json |
| `comms_noaa_radio` | midlandusa.com | Hand-crank emergency weather radio with alerts | 64.99 | 1 | 64.99 | radio | https://midlandusa.com/products/er210-portable-emergency-crank-weather-radio |
| `comms_frs_radios` | midlandusa.com | License-free FRS two-way radios, pair | 74.99 | 1 | 74.99 | pair | https://midlandusa.com/products/lxt600vp4-frs-walkie-talkie-2-pack-silver |
| `comms_frs_radios` | midlandusa.com | License-free FRS two-way radios, pair, longer range model | 89.99 | 1 | 89.99 | pair | https://midlandusa.com/products/t71pnk-x-talker-frs-walkie-talkie-2-pack |
| `docs_document_pouch` | webstaurantstore.com | Fire-resistant document bag, 15 x 11 in | 21.99 | 1 | 21.99 | pouch | https://www.webstaurantstore.com/fire-proof-pouch/10513518X.html |
| `docs_document_pouch` | webstaurantstore.com | Fire-resistant document case, 11 x 15 x 4.5 in | 54.99 | 1 | 54.99 | pouch | https://www.webstaurantstore.com/fire-proof-portable-case/10589407P.html |
| `evac_go_bag` | tetonsports.com | Hiking backpack, about 35 liters | 59.99 | 1 | 59.99 | backpack | https://www.tetonsports.com/products/pursuit-2000-backpack |
| `evac_go_bag` | tetonsports.com | Hiking backpack, 30 liters | 79.99 | 1 | 79.99 | backpack | https://www.tetonsports.com/products/numa-30l-onyx-backpack |
| `evac_whistle` | webstaurantstore.com | Plastic pea whistle | 1.20 | 1 | 1.20 | whistle | https://www.webstaurantstore.com/kemp-usa-black-plastic-pea-whistle-10423blk/89610423BLK.html |
| `evac_whistle` | webstaurantstore.com | Loud plastic safety whistle | 2.50 | 1 | 2.50 | whistle | https://www.webstaurantstore.com/kemp-usa-bengal-60-orange-whistle-10-426-org/89610426ORG.html |
| `evac_shelter_in_place_kit` | webstaurantstore.com | 6-mil plastic sheeting, 4 x 100 ft ($35.99) plus a 60-yard roll of duct tape ($7.49) | 43.48 | 1 | 43.48 | kit | https://www.webstaurantstore.com/lavex-industrial-4-x-100-6-mil-clear-polyethylene-construction-sheeting-on-a-roll/422SH071006C.html |
| `evac_shelter_in_place_kit` | webstaurantstore.com | 6-mil plastic sheeting, 8 x 100 ft ($62.99) plus a 60-yard roll of duct tape ($7.49) | 70.48 | 1 | 70.48 | kit | https://www.webstaurantstore.com/lavex-industrial-8-x-100-6-mil-clear-polyethylene-construction-sheeting-on-a-roll/422SH081006C.html |
| `gethome_bag` | tetonsports.com | Hiking backpack, about 35 liters | 59.99 | 1 | 59.99 | bag | https://www.tetonsports.com/products/pursuit-2000-backpack |
| `gethome_bag` | tetonsports.com | Hiking backpack, 30 liters | 79.99 | 1 | 79.99 | bag | https://www.tetonsports.com/products/numa-30l-onyx-backpack |
| `gethome_car_kit` | mypatriotsupply.com (research supply §9.2) | Car survival kit | 129.95 | 1 | 129.95 | kit | https://mypatriotsupply.com/products.json |
| `gethome_car_kit` | beprepared.com | Car survival kit | 149.74 | 1 | 149.74 | kit | https://beprepared.com/products/essential-car-survival-kit |
| `fire_smoke_alarm` | webstaurantstore.com | Battery-operated compact smoke alarm | 20.49 | 1 | 20.49 | alarm | https://www.webstaurantstore.com/kidde-detect-series-10sdr-battery-operated-compact-smoke-alarm-21031428/ |
| `fire_smoke_alarm` | webstaurantstore.com | Battery-operated smoke alarm | 30.99 | 1 | 30.99 | alarm | https://www.webstaurantstore.com/kidde-detect-series-20sdr-battery-operated-smoke-alarm-210314/ |
| `fire_co_alarm` | webstaurantstore.com | Battery-operated carbon monoxide alarm | 33.99 | 1 | 33.99 | alarm | https://www.webstaurantstore.com/kidde-battery-operated-carbon-monoxide-alarm-21026458/472KNCOBLP2.html |
| `fire_co_alarm` | webstaurantstore.com | Plug-in carbon monoxide alarm with battery backup | 41.49 | 1 | 41.49 | alarm | https://www.webstaurantstore.com/first-alert-plug-in-carbon-monoxide-alarm-faj1039730-120v/218FJA1 |
| `fire_extinguisher` | webstaurantstore.com | 2.5 lb ABC dry chemical extinguisher | 40.49 | 1 | 40.49 | extinguisher | https://www.webstaurantstore.com/buckeye-2-5-lb-abc-dry-chemical-fire-extinguisher-rechargeable-untagged-with-vehicle-bracket-ul-rating-1-a-10-b-c/47213315.html |
| `fire_extinguisher` | webstaurantstore.com | 5 lb ABC dry chemical extinguisher with wall bracket | 45.49 | 1 | 45.49 | extinguisher | https://www.webstaurantstore.com/badger-advantage-adv-550-5-lb-dry-chemical-abc-fire-extinguisher-with-wall-bracket-untagged-and-rechargeable-ul-rating-3-a-40-b-c/472ADV550.html |
| `fire_extinguisher` | webstaurantstore.com | 5 lb ABC dry chemical extinguisher, second model | 48.49 | 1 | 48.49 | extinguisher | https://www.webstaurantstore.com/buckeye-5-lb-abc-dry-chemical-fire-extinguisher-rechargeable-untagged-with-wall-mount-ul-rating-3-a-40-b-c/47210914.html |
| `fire_escape_ladder` | webstaurantstore.com | Two-story escape ladder, 13 ft | 69.99 | 1 | 69.99 | ladder | https://www.webstaurantstore.com/kidde-13-2-story-escape-ladder-468093-1-000-lb-capacity/472468093.html |
| `fire_escape_ladder` | webstaurantstore.com | Three-story escape ladder, 25 ft | 114.99 | 1 | 114.99 | ladder | https://www.webstaurantstore.com/kidde-25-3-story-escape-ladder-468094-1-000-lb-capacity/47246809 |
| `fire_utility_wrench` | webstaurantstore.com | 4.5-inch adjustable wrench | 26.99 | 1 | 26.99 | wrench | https://www.webstaurantstore.com/klein-tools-4-1-2-adjustable-wrench-d506-4/992D5064.html |
| `fire_utility_wrench` | webstaurantstore.com | 8-inch adjustable wrench | 31.99 | 1 | 31.99 | wrench | https://www.webstaurantstore.com/klein-tools-8-extra-capacity-adjustable-wrench-d507-8/992D5078.html |
| `fire_clean_air_room` | levoit.com | Room air purifier with HEPA filter | 119.99 | 1 | 119.99 | air cleaner | https://levoit.com/products/vital100-air-purifier |
| `fire_clean_air_room` | levoit.com | Room air purifier with HEPA filter, larger room | 129.99 | 1 | 129.99 | air cleaner | https://levoit.com/products/levoit-eleva-300-smart-air-purifier |
| `special_pet_kit` | mymedic.com | Pet first-aid kit for dogs | 34.95 | 1 | 34.95 | pet | https://mymedic.com/products/pet-medic-dog-first-aid-kit |
| `special_pet_kit` | preppi.co | Dog emergency go-kit | 100.00 | 1 | 100.00 | pet | https://preppi.co/products/dog-emergency-kit |
| `special_pet_food` | openfarmpet.com | Dry dog food, 22 lb bag | 89.99 | 22 | 4.09 | pound of dry food | https://openfarmpet.com/products/farmstead-duck-ancient-grains-dog-kibble |
| `special_pet_food` | stellaandchewys.com | Dry dog food, 21 lb bag | 79.99 | 21 | 3.81 | pound of dry food | https://www.stellaandchewys.com/products/raw-coated-puppy-kibble-chicken-salmon-superfoods |
| `rare_radiation_meter` | mypatriotsupply.com | Personal radiation dosimeter card | 24.95 | 1 | 24.95 | dosimeter | https://mypatriotsupply.com/products/radtriage50-personal-radiation-dosimeter |
| `rare_radiation_meter` | beprepared.com | Personal radiation dosimeter card, second store | 24.99 | 1 | 24.99 | dosimeter | https://beprepared.com/products/radtriage50-personal-radiation-dosimeter |
| `rare_faraday_storage` | beprepared.com | Shielded (Faraday) storage bag, 15 liters | 37.95 | 1 | 37.95 | bag | https://beprepared.com/products/faraday-bag |
| `rare_faraday_storage` | beprepared.com | Shielded (Faraday) waterproof backpack, 30 liters | 97.95 | 1 | 97.95 | bag | https://beprepared.com/products/waterproof-faraday-backpack |
