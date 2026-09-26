# Price observations

The retail observations behind every price band in `content/items/*.toml`. Each priced item has at
least two observations; its band runs from the lowest to the highest unit price here (the
`rr-content` test `price_bands_match_the_observation_log` checks this). Cited in the app as
`rr_price_observations_2026_09`.

- **Observed 2026-09-25** unless the source says otherwise (a few rows reuse observations from
  `docs/research/supply-standards.md` made the same day, and two use archived store pages). The
  rows under "Polish round" at the end were observed 2026-09-26 and replace every earlier row for
  the same items.
- **Sources** are the stores and agencies that served prices to an automated request: open product
  feeds of specialty retailers, a restaurant and janitorial supplier, and federal price series (BLS
  average prices, USDA food plans) and survey averages (KFF copays). In the first pass big-box stores
  blocked automated requests, so everyday prices were under-represented and several bands ran high.
  The polish round re-priced the 30 most expensive bands from a home center's and a supermarket's
  search pages, read in an ordinary browser, and from makers' list prices; stores that answered with
  a bot check were skipped, never worked around. Each item's note says what its band reflects.
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
| `food_cooking_fuel` | webstaurantstore.com | Butane fuel, 8 oz canisters, pack of 48 | 69.99 | 48 | 1.46 | 8-ounce canister | https://www.webstaurantstore.com/choice-butane-fuel-refill-8-oz-canister-pack/174BUTANE48KIT.html |
| `food_cooking_fuel` | webstaurantstore.com | Butane fuel, 8 oz canisters, case of 12 | 32.49 | 12 | 2.71 | 8-ounce canister | https://www.webstaurantstore.com/sterno-products-50162-butane-fuel-refill-8-oz-canister-case/22350162.html |
| `food_fridge_thermometers` | webstaurantstore.com | Dial refrigerator/freezer thermometer (two needed) | 2.39 | 0.5 | 4.78 | pair | https://www.webstaurantstore.com/choice-2-dial-refrigerator-freezer-thermometer/9142DRFT.html |
| `food_fridge_thermometers` | webstaurantstore.com | Digital refrigerator/freezer thermometer (two needed) | 5.29 | 0.5 | 10.58 | pair | https://www.webstaurantstore.com/avatemp-digital-refrigerator-freezer-thermometer/914FTTHERM.html |
| `med_reserve_supply` | kff.org | Average copay for a first-tier (generic) drug, employer plans, 2025: $12 per 30-day fill | 12.00 | 30 | 0.40 | day of one person's medicine | https://www.kff.org/health-costs/2025-employer-health-benefits-survey/ |
| `med_reserve_supply` | kff.org | Average copay for a second-tier (preferred brand) drug, employer plans, 2025: $40 per 30-day fill | 40.00 | 30 | 1.33 | day of one person's medicine | https://www.kff.org/health-costs/2025-employer-health-benefits-survey/ |
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
| `thermal_battery_fan` | webstaurantstore.com | Rechargeable handheld fan | 10.99 | 1 | 10.99 | fan | https://www.webstaurantstore.com/visionair-4-black-rechargeable-battery-operated-3-speed-usb-handheld-fan/ |
| `thermal_battery_fan` | o2cool.com | Battery desk and handheld fan | 22.99 | 1 | 22.99 | fan | https://www.o2cool.com/products/handy-fan |
| `thermal_indoor_thermometer` | webstaurantstore.com | Stick-on dial indoor/outdoor thermometer | 2.19 | 1 | 2.19 | thermometer | https://www.webstaurantstore.com/taylor-5380n-1-3-4-mini-window-stick-on-indoor-outdoor-thermometer/ |
| `thermal_indoor_thermometer` | webstaurantstore.com | 12-inch wall thermometer with humidity dial | 15.49 | 1 | 15.49 | thermometer | https://www.webstaurantstore.com/taylor-497j-12-dial-indoor-outdoor-wall-thermometer-with-hygrometer/ |
| `thermal_emergency_blankets` | webstaurantstore.com | Foil emergency blanket | 0.50 | 1 | 0.50 | blanket | https://www.webstaurantstore.com/kemp-usa-mylar-foil-emergency-thermal-blanket-10-601/89610601.html |
| `thermal_emergency_blankets` | beprepared.com | Emergency blanket | 4.95 | 1 | 4.95 | blanket | https://beprepared.com/products/z_legacy_z_cc_02_emergency-blanket |
| `power_batteries` | webstaurantstore.com | AA alkaline batteries, pack of 24 (industrial) | 10.49 | 1 | 10.49 | 24-pack | https://www.webstaurantstore.com/rayovac-alaa-24ppj-ultra-pro-industrial-aa-alkaline-batteries-24-pack/ |
| `power_batteries` | webstaurantstore.com | AA alkaline batteries, pack of 24 (name brand) | 19.99 | 1 | 19.99 | 24-pack | https://www.webstaurantstore.com/energizer-max-e91bp-24-aa-alkaline-batteries-pack/199E91AAP24.html |
| `power_generator_fuel` | bls.gov | Regular gasoline, US city average, August 2026 | 4.20 | 1 | 4.20 | gallon | https://data.bls.gov/timeseries/APU000074714 |
| `power_generator_fuel` | bls.gov | Regular gasoline, US city average, July 2026 | 4.09 | 1 | 4.09 | gallon | https://data.bls.gov/timeseries/APU000074714 |
| `evac_whistle` | webstaurantstore.com | Plastic pea whistle | 1.20 | 1 | 1.20 | whistle | https://www.webstaurantstore.com/kemp-usa-black-plastic-pea-whistle-10423blk/89610423BLK.html |
| `evac_whistle` | webstaurantstore.com | Loud plastic safety whistle | 2.50 | 1 | 2.50 | whistle | https://www.webstaurantstore.com/kemp-usa-bengal-60-orange-whistle-10-426-org/89610426ORG.html |
| `rare_radiation_meter` | mypatriotsupply.com | Personal radiation dosimeter card | 24.95 | 1 | 24.95 | dosimeter | https://mypatriotsupply.com/products/radtriage50-personal-radiation-dosimeter |
| `rare_radiation_meter` | beprepared.com | Personal radiation dosimeter card, second store | 24.99 | 1 | 24.99 | dosimeter | https://beprepared.com/products/radtriage50-personal-radiation-dosimeter |
| `rare_faraday_storage` | beprepared.com | Shielded (Faraday) storage bag, 15 liters | 37.95 | 1 | 37.95 | bag | https://beprepared.com/products/faraday-bag |
| `rare_faraday_storage` | beprepared.com | Shielded (Faraday) waterproof backpack, 30 liters | 97.95 | 1 | 97.95 | bag | https://beprepared.com/products/waterproof-faraday-backpack |

## Polish round (observed 2026-09-26)

These rows replace the earlier observations for the same items. Home-center and supermarket listings
were read from their search pages in an ordinary browser (both served prices without a bot check;
two other chains challenged the browser, and those pages were not used). Makers' list prices come from
their open product feeds. Composite rows add two listings bought together, as before.

| Item | Source | Listing | Listed price (USD) | Units | Unit price (USD) | Per | URL |
|---|---|---|---|---|---|---|---|
| `fire_co_alarm` | homedepot.com (browser) | Battery carbon monoxide alarm (model 21030863) | 23.47 | 1 | 23.47 | alarm | https://www.homedepot.com/s/carbon%20monoxide%20detector%20battery |
| `fire_co_alarm` | homedepot.com (browser) | AA-battery carbon monoxide alarm with digital display (model 21033773) | 29.97 | 1 | 29.97 | alarm | https://www.homedepot.com/s/carbon%20monoxide%20detector%20battery |
| `fire_smoke_alarm` | homedepot.com (browser) | AA-battery compact photoelectric smoke alarm (model 21031428) | 18.97 | 1 | 18.97 | alarm | https://www.homedepot.com/s/smoke%20detector%20battery%20operated |
| `fire_smoke_alarm` | homedepot.com (browser) | 10-year sealed-battery smoke alarm (model 21031466) | 32.47 | 1 | 32.47 | alarm | https://www.homedepot.com/s/smoke%20detector%20battery%20operated |
| `fire_extinguisher` | homedepot.com (browser) | A-B-C 1-A:10-B:C 2.5 lb home extinguisher, two-pack (model 21030932) | 44.97 | 2 | 22.49 | extinguisher | https://www.homedepot.com/s/fire%20extinguisher%20ABC%20home |
| `fire_extinguisher` | homedepot.com (browser) | A-B-C 1-A:10-B:C 2.5 lb rechargeable extinguisher (model 21030922) | 44.47 | 1 | 44.47 | extinguisher | https://www.homedepot.com/s/fire%20extinguisher%20ABC%20home |
| `fire_escape_ladder` | homedepot.com (browser) | Two-story 13 ft escape ladder (model 21030942) | 59.97 | 1 | 59.97 | ladder | https://www.homedepot.com/s/fire%20escape%20ladder%202%20story |
| `fire_escape_ladder` | homedepot.com (browser) | Retractable 13 ft two-story aluminum escape ladder | 58.99 | 1 | 58.99 | ladder | https://www.homedepot.com/s/fire%20escape%20ladder%202%20story |
| `fire_utility_wrench` | homedepot.com (browser) | Emergency gas and water shutoff tool (model 26097) | 11.63 | 1 | 11.63 | wrench | https://www.homedepot.com/s/gas%20shut%20off%20wrench |
| `fire_utility_wrench` | homedepot.com (browser) | Gas and water service shut-off wrench (model 2750) | 21.99 | 1 | 21.99 | wrench | https://www.homedepot.com/s/gas%20shut%20off%20wrench |
| `fire_clean_air_room` | homedepot.com (browser) | HEPA room air cleaner, about 220 sq ft (Core 300-P) | 99.99 | 1 | 99.99 | air cleaner | https://www.homedepot.com/s/hepa%20air%20purifier%20room |
| `fire_clean_air_room` | homedepot.com (browser) | True HEPA room air cleaner, 360 sq ft (D360) | 149.00 | 1 | 149.00 | air cleaner | https://www.homedepot.com/s/hepa%20air%20purifier%20room |
| `power_headlamp` | homedepot.com (browser) | 200-lumen LED headlamps, three-pack (model 90651) | 9.97 | 3 | 3.32 | headlamp | https://www.homedepot.com/s/led%20headlamp |
| `power_headlamp` | homedepot.com (browser) | 300-lumen LED headlamp (model HDB32EH) | 17.47 | 1 | 17.47 | headlamp | https://www.homedepot.com/s/led%20headlamp |
| `power_headlamp` | homedepot.com (browser) | 400-lumen rechargeable headlamp (model PVL-HLP-0004) | 19.97 | 1 | 19.97 | headlamp | https://www.homedepot.com/s/led%20headlamp |
| `power_lantern` | homedepot.com (browser) | Rechargeable solar LED lantern (model GG-113-30LSPOP) | 21.82 | 1 | 21.82 | lantern | https://www.homedepot.com/s/rechargeable%20led%20lantern |
| `power_lantern` | homedepot.com (browser) | LED lantern with USB phone-charging port (model XPCW19B) | 24.99 | 1 | 24.99 | lantern | https://www.homedepot.com/s/rechargeable%20led%20lantern |
| `power_bank` | iniushop.com | 20,000 mAh 22.5 W power bank (maker's list price) | 32.99 | 1 | 32.99 | power bank | https://www.iniushop.com/products/iniu-carry-p512-power-bank-22-5w-smallest-20000mah |
| `power_bank` | iniushop.com | 20,000 mAh 20 W power bank (maker's list price) | 39.99 | 1 | 39.99 | power bank | https://www.iniushop.com/products/iniu-portable-charger-compact-20000mah-pd-20w-power-bank |
| `power_station` | homedepot.com (browser) | 1,024 Wh portable power station (model EL100V2) | 479.00 | 1 | 479.00 | power station | https://www.homedepot.com/s/portable%20power%20station%201000wh |
| `power_station` | homedepot.com (browser) | 1,030 Wh power station (model PP1030i) | 487.90 | 1 | 487.90 | power station | https://www.homedepot.com/s/portable%20power%20station%201000wh |
| `power_generator` | homedepot.com (browser) | 2,300 W gasoline inverter generator (model GXS2300i) | 389.99 | 1 | 389.99 | generator | https://www.homedepot.com/s/inverter%20generator%202200%20watt |
| `power_generator` | homedepot.com (browser) | 2,800 W gasoline inverter generator (model iGen2800c) | 499.00 | 1 | 499.00 | generator | https://www.homedepot.com/s/inverter%20generator%202200%20watt |
| `power_solar_panel` | jackery.com | 100 W folding solar panel (maker's list price) | 199.00 | 1 | 199.00 | panel | https://www.jackery.com/products/jackery-solarsaga-100-prime-solar-panel |
| `power_solar_panel` | us.ecoflow.com | 110 W portable solar panel (maker's list price) | 259.00 | 1 | 259.00 | panel | https://us.ecoflow.com/products/110w-portable-solar-panel |
| `power_device_battery` | us.ecoflow.com | 256 Wh portable power station (maker's list price) | 209.00 | 1 | 209.00 | battery | https://us.ecoflow.com/products/river-2-240-portable-power-station |
| `power_device_battery` | jackery.com | About 290 Wh portable power station (maker's list price) | 279.00 | 1 | 279.00 | battery | https://www.jackery.com/products/jackery-explorer-300-v2-portable-power-station |
| `comms_noaa_radio` | homedepot.com (browser) | Clock radio with NOAA weather alert (model JEP-725) | 36.99 | 1 | 36.99 | radio | https://www.homedepot.com/s/noaa%20weather%20radio |
| `comms_noaa_radio` | homedepot.com (browser) | Portable AM/FM weather radio with NOAA alert and flashlight (model JEP-775) | 42.50 | 1 | 42.50 | radio | https://www.homedepot.com/s/noaa%20weather%20radio |
| `comms_frs_radios` | homedepot.com (browser) | Rechargeable license-free two-way radios, pair (model T210) | 38.98 | 1 | 38.98 | pair | https://www.homedepot.com/s/two%20way%20radio%20walkie%20talkie |
| `comms_frs_radios` | homedepot.com (browser) | Rechargeable license-free two-way radios, pair (model T270) | 44.99 | 1 | 44.99 | pair | https://www.homedepot.com/s/two%20way%20radio%20walkie%20talkie |
| `evac_go_bag` | homedepot.com (browser) | 17 in. classic backpack (model B12A) | 29.73 | 1 | 29.73 | bag | https://www.homedepot.com/s/backpack |
| `evac_go_bag` | homedepot.com (browser) | 14 in. four-compartment backpack (model 49020) | 38.80 | 1 | 38.80 | bag | https://www.homedepot.com/s/backpack |
| `gethome_bag` | homedepot.com (browser) | 17 in. classic backpack (model B12A) | 29.73 | 1 | 29.73 | bag | https://www.homedepot.com/s/backpack |
| `gethome_bag` | homedepot.com (browser) | 14 in. four-compartment backpack (model 49020) | 38.80 | 1 | 38.80 | bag | https://www.homedepot.com/s/backpack |
| `evac_shelter_in_place_kit` | homedepot.com (browser) | Composite: 10 x 25 ft 6-mil plastic sheeting ($21.10) + 60 yd duct tape ($6.98) | 28.08 | 1 | 28.08 | kit | https://www.homedepot.com/s/6%20mil%20plastic%20sheeting%2010%20ft%20x%2025%20ft |
| `evac_shelter_in_place_kit` | homedepot.com (browser) | Composite: 10 x 50 ft 6-mil plastic sheeting ($28.70) + 60 yd duct tape ($6.98) | 35.68 | 1 | 35.68 | kit | https://www.homedepot.com/s/duct%20tape |
| `gethome_car_kit` | homedepot.com (browser) | 55-piece roadside tool and first-aid kit with bag (model 75-EMG1053) | 34.97 | 1 | 34.97 | kit | https://www.homedepot.com/s/roadside%20emergency%20kit |
| `gethome_car_kit` | homedepot.com (browser) | Auto emergency response kit (model 70352) | 87.89 | 1 | 87.89 | kit | https://www.homedepot.com/s/roadside%20emergency%20kit |
| `docs_document_pouch` | homedepot.com (browser) | Fireproof and waterproof document bag (model 80003-B) | 22.98 | 1 | 22.98 | pouch | https://www.homedepot.com/s/fireproof%20document%20bag |
| `docs_document_pouch` | homedepot.com (browser) | Fire- and water-resistant document bag, medium (model FB1511) | 33.15 | 1 | 33.15 | pouch | https://www.homedepot.com/s/fireproof%20document%20bag |
| `san_twin_bucket_toilet` | homedepot.com (browser) + relianceoutdoors.com | Composite: two 5-gallon buckets ($3.98 each) + snap-on bucket toilet seat and cover ($17.99 maker's list price) | 25.95 | 1 | 25.95 | kit | https://relianceoutdoors.com/products/luggable-loo-seat-cover |
| `san_twin_bucket_toilet` | relianceoutdoors.com + homedepot.com (browser) | Composite: bucket toilet with seat ($25.99 maker's list price) + a second 5-gallon bucket ($3.98) | 29.97 | 1 | 29.97 | kit | https://relianceoutdoors.com/products/luggable-loo-portable-toilet |
| `san_baby_wipes` | kroger.com (browser) | Fragrance-free baby wipes, 72 count (store brand) | 2.29 | 1 | 2.29 | pack | https://www.kroger.com/search?query=baby%20wipes |
| `san_baby_wipes` | kroger.com (browser) | Unscented sensitive baby wipes, 56 count | 2.69 | 1 | 2.69 | pack | https://www.kroger.com/search?query=baby%20wipes |
| `food_camp_stove` | homedepot.com (browser) | One-burner butane stove, 7,650 BTU (model GS-1000G-H) | 26.35 | 1 | 26.35 | stove | https://www.homedepot.com/s/butane%20camp%20stove |
| `food_camp_stove` | homedepot.com (browser) | Classic one-burner butane camping stove (model 2157595) | 39.99 | 1 | 39.99 | stove | https://www.homedepot.com/s/butane%20camp%20stove |
| `food_cooler` | homedepot.com (browser) | 50 qt chest cooler (model 51179) | 49.98 | 1 | 49.98 | cooler | https://www.homedepot.com/s/igloo%20cooler%20quart |
| `food_cooler` | homedepot.com (browser) | 52 qt hard cooler (model 2182646, online price) | 54.99 | 1 | 54.99 | cooler | https://www.homedepot.com/s/cooler%2048%20quart |
| `food_infant_formula` | kroger.com (browser) | Infant formula powder, store brand, 34 oz at the regular price ($31.99 on sale) | 36.99 | 34 | 1.09 | ounce of powder | https://www.kroger.com/search?query=infant%20formula%20powder |
| `food_infant_formula` | kroger.com (browser) | Infant formula powder with iron, name brand, 30.8 oz | 47.99 | 30.8 | 1.56 | ounce of powder | https://www.kroger.com/search?query=infant%20formula%20powder |
| `food_cooking_pot` | kroger.com (browser) | 12 qt stainless stock pot with lid | 17.99 | 1 | 17.99 | pot | https://www.kroger.com/search?query=stock%20pot%20with%20lid |
| `food_cooking_pot` | webstaurantstore.com | 8 qt aluminum stock pot with cover | 28.99 | 1 | 28.99 | pot | https://www.webstaurantstore.com/search/8-qt-stock-pot-with-cover.html |
| `food_three_days_basic` | fns.usda.gov | Thrifty Food Plan, reference family of four, August 2026: $236.30 a week = $8.44 a person-day, x 3 days | 236.30 | 9.333 | 25.32 | 3 days of food for one person | https://www.fns.usda.gov/sites/default/files/resource-files/CostofFoodAug2026Thrifty.pdf |
| `food_three_days_basic` | fns.usda.gov | Low-Cost Food Plan, male 19-50, August 2026: $73.20 a week = $10.46 a person-day, x 3 days | 73.20 | 2.333 | 31.37 | 3 days of food for one person | https://www.fns.usda.gov/sites/default/files/resource-files/CostofFoodAug2026LowModLib.pdf |
| `med_first_aid_kit` | homedepot.com (browser) | 260-piece first-aid kit (model 52246) | 27.47 | 1 | 27.47 | kit | https://www.homedepot.com/s/first%20aid%20kit |
| `med_first_aid_kit` | homedepot.com (browser) | 328-piece ANSI-rated first-aid kit (model 52247) | 43.97 | 1 | 43.97 | kit | https://www.homedepot.com/s/first%20aid%20kit |
| `med_thermometer` | kroger.com (browser) | Flexible-tip digital thermometer (store brand) | 7.49 | 1 | 7.49 | thermometer | https://www.kroger.com/search?query=digital%20thermometer |
| `med_thermometer` | kroger.com (browser) | Flexible-tip digital thermometer (second store brand) | 8.49 | 1 | 8.49 | thermometer | https://www.kroger.com/search?query=digital%20thermometer |
| `thermal_sleeping_bag` | homedepot.com (browser) | Three-season mummy sleeping bag (model GER-1134) | 43.48 | 1 | 43.48 | sleeping bag | https://www.homedepot.com/s/sleeping%20bag |
| `thermal_sleeping_bag` | homedepot.com (browser) | Adult cold-weather mummy sleeping bag (model 2205651) | 94.49 | 1 | 94.49 | sleeping bag | https://www.homedepot.com/s/sleeping%20bag |
| `thermal_blankets` | webstaurantstore.com | Twin 66 x 90 in. fleece blanket, case of 4 | 57.49 | 4 | 14.37 | blanket | https://www.webstaurantstore.com/search/fleece-blanket.html |
| `thermal_blankets` | kroger.com (browser) | 50 x 60 in. microplush throw at the regular price ($9.99 on sale) | 19.98 | 1 | 19.98 | blanket | https://www.kroger.com/search?query=fleece%20throw%20blanket |
| `thermal_warm_layers` | webstaurantstore.com | Composite: cuffed knit beanie ($7.39) + insulated thermal gloves ($8.79) | 16.18 | 1 | 16.18 | set | https://www.webstaurantstore.com/search/knit-beanie.html |
| `thermal_warm_layers` | webstaurantstore.com | Composite: acrylic knit watch cap ($12.50) + insulated thermal gloves ($9.19) | 21.69 | 1 | 21.69 | set | https://www.webstaurantstore.com/search/winter-gloves.html |
| `thermal_cooling_towel` | homedepot.com (browser) | Cotton bath towels, four-piece set | 29.99 | 4 | 7.50 | towel | https://www.homedepot.com/s/bath%20towel |
| `thermal_cooling_towel` | homedepot.com (browser) | Oversized microfiber bath towels, set of 2 | 27.68 | 2 | 13.84 | towel | https://www.homedepot.com/s/bath%20towel |
| `special_pet_kit` | homedepot.com (browser) | Medium soft canvas pet carrier (model B21BLMD) | 29.93 | 1 | 29.93 | pet | https://www.homedepot.com/s/pet%20carrier |
| `special_pet_kit` | homedepot.com (browser) | Medium soft pet carrier (model B18BKPMD) | 34.99 | 1 | 34.99 | pet | https://www.homedepot.com/s/pet%20carrier |
| `special_pet_food` | kroger.com (browser) | Adult dry dog food, 38 lb bag | 27.99 | 38 | 0.74 | pound of dry food | https://www.kroger.com/search?query=dry%20dog%20food%2030%20lb |
| `special_pet_food` | kroger.com (browser) | Adult dry dog food, 13 lb bag | 22.49 | 13 | 1.73 | pound of dry food | https://www.kroger.com/search?query=dry%20dog%20food%2030%20lb |
| `special_nursing_supplies` | lansinoh.com | Composite: manual breast pump ($34.99) + disposable nursing pads ($11.99), makers' list prices | 46.98 | 1 | 46.98 | kit | https://lansinoh.com/products.json |
| `special_nursing_supplies` | lansinoh.com + drbrownsbaby.com | Composite: manual breast pump ($34.99) + disposable breast pads ($9.99), makers' list prices | 44.98 | 1 | 44.98 | kit | https://www.drbrownsbaby.com/products.json |
| `water_livestock_tank` | homedepot.com (browser) | 350-gallon poly stock tank (model WTDT62G) | 397.00 | 350 | 1.13 | gallon of tank space | https://www.homedepot.com/s/rubbermaid%20stock%20tank |
| `water_livestock_tank` | homedepot.com (browser) | 100-gallon galvanized oval stock tank (model WT224) | 159.98 | 100 | 1.60 | gallon of tank space | https://www.homedepot.com/s/100%20gallon%20stock%20tank |
