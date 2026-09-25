# Supply-Sizing Evidence Base for a Household Disaster-Preparedness Planner

Compiled 2026-09-25. Scope: per-person-day and per-household quantities, adjustment factors, shelf lives, and 2025-2026 USD cost benchmarks, each traced to a source.

## 0. Conventions (read first)

| Tag | Meaning |
|---|---|
| **[PD-quote]** | Exact wording from a U.S. federal source. These are public domain (17 U.S.C. 105), so they can be quoted verbatim in the app. |
| **[paraphrase]** | Copyrighted or non-federal source: Sphere, WHO, the Church of Jesus Christ of Latter-day Saints, BYU, USU, the Red Cross, ASPCA, state agencies, journals, vendors, and the EFFAK (which is copyright Operation HOPE). Numbers are reported as facts; follow the URL for the exact wording before quoting it in the UI. |
| **DERIVED** | My own arithmetic from cited inputs. The formula is shown. |
| **UNVERIFIED** | I could not confirm this against a primary source in this pass. Do not ship it as a sourced number. |
| **Access note** | Ready.gov, FEMA.gov, FSIS, FCC, RedCross.org and spherestandards.org block automated fetching. Their content came from WebFetch, from Internet Archive (Wayback) captures (capture timestamp given), or from official mirrors (ReliefWeb for Sphere). Re-verify before release. |
| **Prices** | Observed on vendor pages or Wayback captures on the date stated. Retail prices are volatile, so store them with an as-of date and re-scrape periodically. |

---

## 1. WATER

### 1.1 Baseline quantity per person per day

| Authority | Quantity | Duration it pairs with | Wording / paraphrase | Source |
|---|---|---|---|---|
| Ready.gov (Water page, updated 02/22/2021) | **1 US gal (3.79 L)** | "several days" | [PD-quote] "Store at least one gallon of water per person per day for several days, for drinking and sanitation." / "A normally active person needs about three quarters of a gallon of fluid daily, from water and other beverages." | https://www.ready.gov/water |
| Ready.gov Build a Kit (updated 07/01/2026) | 1 gal | several days | [PD-quote] "Water (one gallon per person per day for several days, for drinking and sanitation)" | https://www.ready.gov/kit |
| Ready.gov Build a Kit, **historical** (Wayback 2020-01-01 and 2021-07-01) | 1 gal | **at least 3 days / 72 h** | [PD-quote] "one gallon of water per person per day for at least three days" and "supplies to last for at least 72 hours". The wording changed to "several days" between the 2021-07-01 and 2022-07-01 captures. | https://web.archive.org/web/20200101090440/https://www.ready.gov/kit ; https://web.archive.org/web/20220701233904/https://www.ready.gov/kit |
| CDC, "How to Create an Emergency Water Supply" (June 27, 2025) | 1 gal | 3 days, "try" 2 weeks | [PD-quote] "Store at least 1 gallon of water per person, per day for 3 days... Try to store a 2-week supply if possible." / "Consider storing more water than this for pregnant women, people who are sick, pets, or if you live in a hot climate." | https://www.cdc.gov/water-emergency/about/how-to-create-and-store-an-emergency-water-supply.html |
| American Red Cross, Survival Kit Supplies (Wayback 2026-09-20) | 1 gal | **3-day supply for evacuation, 2-week supply at home** | [paraphrase] | https://web.archive.org/web/20260920051259/https://www.redcross.org/get-help/how-to-prepare-for-emergencies/survival-kit-supplies.html |
| Washington EMD, *Prepare in a Year* guide | 1 gal (drinking, cooking, hygiene) | at least 2 weeks (14 gal/person) | [paraphrase] Drink at least **1 quart** per person per day. You may need less than 1 gal depending on cooking and on using wet wipes. | https://mil.wa.gov/asset/5f171cc0a935f (via https://mil.wa.gov/prepare-in-a-year) |
| Oregon OEM, *Be 2 Weeks Ready* tool kit | 1 gal | 2 weeks | [paraphrase] Track how much water your pet actually uses and store that. Water weighs about 8.3 lb/gal. | https://www.oregon.gov/oem/Documents/B2WR-Complete-Tool-Kit-EN.pdf |
| Church of Jesus Christ, *Emergency Preparedness* manual (2023) | 1 gal/adult total; **≥2 quarts** drinking | 2 weeks = **14 gal (53 L)/adult** | [paraphrase] | https://www.churchofjesuschrist.org/study/manual/emergency-preparedness/01-food-and-water-storage/02-guidelines-on-emergency-water-storage-and-purification?lang=eng |
| **Sphere Handbook 2018** (4th ed., still current; a 5th ed. is reportedly expected around 2028, UNVERIFIED), WASH Water Supply Std 2.1 | **15 L (≈4.0 gal)** household minimum | humanitarian established practice | [paraphrase] 15 L/p/d is a floor, not a ceiling. **7.5 L** may be acceptable briefly in the acute phase of a drought. **50 L** may be the minimum in urban middle-income settings. | https://reliefweb.int/attachments/fc910c0d-e277-3f9e-930e-7c1dfc8182e8/Sphere-Handbook-2018-EN.pdf (pp. 106-107) |
| Sphere 2018, survival table (Water Supply 2.1 & WASH Appendix 3) | drinking+food **2.5-3 L**; hygiene **2-6 L**; basic cooking **3-6 L**; **total 7.5-15 L** | n/a | [paraphrase] The drinking share depends on climate and physiology. The hygiene and cooking shares depend on norms and food type. | same, pp. 107, 144 |
| WHO/WEDC Technical Note 9 (updated July 2013) | same 7.5-15 L table; **20 L/capita/day** for minimum essential health and hygiene | n/a | [paraphrase] Adapted from Sphere. Research indicates about 20 L is the minimum safe quantity for essential health and hygiene, so supply should rise toward it. | https://cdn.who.int/media/docs/default-source/wash-documents/who-tn-09-how-much-water-is-needed.pdf |
| Germany BBK, *Ratgeber für Notfallvorsorge* (2nd ed., 11/2025) | **2 L** (of which 0.5 L for cooking) | 10 days (at least 3) | [paraphrase] Covers drinking and cooking only. | https://www.bbk.bund.de/SharedDocs/Downloads/DE/Mediathek/Publikationen/Buergerinformationen/Ratgeber/ratgeber-notfallvorsorge.pdf?__blob=publicationFile |
| Denmark BRS, "Prepared for crises" | **3 L** drinking water | 3 days | [paraphrase] | https://www.brs.dk/en/prepared/ |

**Physiological reference: National Academies DRI, Adequate Intake for *total* water (food + beverages).** Source: IOM 2005, *DRIs for Water, Potassium, Sodium, Chloride, and Sulfate*, ch. 4 (NAP 10925). Numbers only. Retrieved via https://web.archive.org/web/2019/https://www.nap.edu/read/10925/chapter/6

| Life stage | Total water AI (L/day) | of which beverages incl. drinking water (L/day) |
|---|---|---|
| Infant 0-6 mo | 0.7 (from human milk) | — |
| Infant 7-12 mo | 0.8 | ~0.6 total fluid |
| Child 1-3 y | 1.3 | 0.9 |
| Child 4-8 y | 1.7 | 1.2 |
| Boys 9-13 / 14-18 | 2.4 / 3.3 | 1.8 / 2.6 |
| Girls 9-13 / 14-18 | 2.1 / 2.3 | 1.6 / 1.8 |
| Men 19+ | 3.7 | 3.0 |
| Women 19+ | 2.7 | 2.2 |
| Pregnancy (14-50 y) | 3.0 | 2.3 |
| Lactation (14-50 y) | 3.8 | 3.1 |

### 1.2 Adjustment factors

| Factor | Evidence | Suggested engine treatment |
|---|---|---|
| Hot climate / exertion | Ready.gov [PD-quote]: "In very hot temperatures, water needs can double." The CDC Yellow Book 2026 heat chapter [paraphrase] says sweat rates may reach about 1 L/hour, but forcing fluids beyond thirst risks hyponatremia. NIOSH (July 16, 2026) [PD-quote]: "For moderate activities in the heat that last less than 2 hours, drink 1 cup (8 oz.) of water every 15–20 minutes" and "fluid intake should not exceed 6 cups per hour." | DERIVED: in hot climates, multiply the drinking share (0.75 gal) by 2 and keep sanitation at 0.25 gal, giving **1.75 gal/p/d**. The conservative option doubles the whole allowance to **2.0 gal/p/d**. |
| Pregnancy / lactation | CDC says store more (above). DRI total water AI is 3.0 L pregnant and 3.8 L lactating, against 2.7 L for non-pregnant women. | DERIVED: add about **+0.3 L/day** for pregnancy and **+1.1 L/day** for lactation (the DRI difference) on top of the base allowance. |
| Children | Ready.gov [PD-quote]: "Children, nursing mothers and sick people may need more water." DRI AIs scale with age (table above). | Keep 1 gal/child as the planning allowance. It already exceeds the DRI for children, and hygiene needs are not lower. |
| Illness / diarrhea | Ready.gov and CDC say "sick people" need more. CDC (cholera page) says ORS powder is mixed with boiled or treated water. The CDC Yellow Book says one packet goes into the indicated volume, generally 1 L (see 3.8). | DERIVED: per anticipated diarrheal illness-day, add **1-2 L** (UNVERIFIED as a household standard; the per-packet water volume is sourced). |
| Infant formula | CDC: use ready-to-feed formula if possible. Otherwise mix powder or concentrate with bottled water, or with water boiled for 1 minute and cooled (§2.5). AAP: at most 32 oz of formula per 24 h. | DERIVED: for powdered formula, budget **≈1 qt (0.95 L) of safe water per infant per day** (max 32 oz prepared), plus water for cleaning feeding items. |
| Rehydrating food | ReadyWise 72-Hour 4-pack spec [paraphrase]: 21,840 kcal (12 person-days at 1,820 kcal) needs about 120 cups of water. | DERIVED: dehydrated or freeze-dried diets add about **0.6 gal/person-day** (7.5 gal / 12 person-days). Add this only when the food plan uses dehydrated food. |
| Wound irrigation | WMS 2014 wound guideline [paraphrase]: increasing irrigant from 0.1 L to 1 L improved bacterial removal, but going up to 10 L added nothing. | Optional: about **1 L of potable water per significant wound** in the first-aid plan. |
| Pets: dogs | PetMD [paraphrase]: about **1 oz of water per lb of body weight per day**. The Merck Vet Manual maintenance-fluid formula is **132 × BW(kg)^0.75 mL/day** (or 30×kg + 70). | Use 1 oz/lb/day. DERIVED checks: 30-lb dog ≈ 0.9 L/day; 60-lb ≈ 1.6-1.8 L/day. |
| Pets: cats | PetMD [paraphrase]: about 4 oz per 5 lb per day. Merck: 80 × BW(kg)^0.75 mL/day. | 10-lb cat ≈ 8 oz (≈0.25 L)/day |
| Livestock | Sphere Appendix 3 [paraphrase]: **20-30 L per large or medium animal per day; 5 L per small animal per day**. | Use directly. Horses and cattle often need more in heat (UNVERIFIED specific figure). |
| Pet kits (ASPCA) | [paraphrase] At least 7 days of bottled water for each person *and* pet, replaced every two months. | Conflicts with the 6-month rotation below; see §13. |

Links: https://www.ready.gov/water · https://www.cdc.gov/niosh/heat-stress/recommendations/index.html · https://www.cdc.gov/yellow-book/hcp/environmental-hazards-risks/heat-and-cold-illness-in-travelers.html · https://www.petmd.com/dog/nutrition/evr_dg_the_importance_of_water · https://www.merckvetmanual.com/therapeutics/fluid-therapy/maintenance-fluid-plan-in-animals · https://www.aspca.org/pet-care/general-pet-care/disaster-preparedness · WMS: https://web.archive.org/web/20200709231429id_/https://www.wildmedcenter.com/uploads/5/9/8/2/5982510/wms_wound_care_12-2014.pdf

### 1.3 Storage and rotation

| Rule | Source |
|---|---|
| [PD-quote] "Observe the expiration date for store-bought water." / "If you are filling containers with water to store, replace the water every 6 months." / Store a bottle of unscented bleach "(label should say it contains between 5% and 9% of sodium hypochlorite)". | CDC 2025 (above) |
| Use FDA-approved food-grade containers. If none, use a tight-closing, unbreakable (not glass) container, preferably narrow-necked. Never use containers that held toxic chemicals. | CDC 2025 |
| Sanitize containers: [PD-quote] "mixing 1 teaspoon of unscented liquid household chlorine bleach in 1 quart (4 cups) of water"; shake to coat; wait at least 30 seconds; pour out; air-dry. | CDC 2025 |
| Label as drinking water with the date. Keep at **50-70°F**, out of sunlight, away from gasoline and pesticides. | CDC 2025 |
| Ready.gov [PD-quote]: "Water that has not been commercially bottled should be replaced every six months." | https://www.ready.gov/water |
| Church manual [paraphrase]: avoid milk jugs (they seal poorly and become brittle). Chlorinated tap water needs no additives. Non-chlorinated clear water gets **8 drops** of 5-9% bleach per gallon; cloudy water gets **16 drops**. Rotate by best-by date or every 6 months. | Church water page (above) |
| WA EMD [paraphrase]: tap water needs nothing added before storage. Rotate stored tap water every 6 months. | WA guide |
| DERIVED weight: 1 gal ≈ 8.34 lb. Per person: 3 days = 25 lb; 14 days = 117 lb; 30 days = 250 lb; 90 days = 751 lb. | arithmetic |

### 1.4 Treatment

**Boiling**
- CDC (Sept 19, 2024) [PD-quote]: "Bring clear water to a rolling boil for 1 minute (at elevations above 6,500 feet, boil for 3 minutes)." Filter or settle cloudy water first. https://www.cdc.gov/water-emergency/about/index.html
- EPA (Feb 24, 2026) [PD-quote]: "Bring water to a rolling boil for at least one minute. At altitudes above 5,000 feet (1,000 meters), boil water for three minutes." https://www.epa.gov/ground-water-and-drinking-water/emergency-disinfection-drinking-water
- Ready.gov [PD-quote]: "Bring water to a rolling boil for one full minute".
- The Church manual [paraphrase] says to boil for 3-5 minutes. This conflicts with the federal guidance; see §13.

**Household bleach (sodium hypochlorite)**

| Water volume | EPA: 6% bleach | EPA: 8.25% bleach | CDC: 5-9% bleach | CDC: 1% bleach |
|---|---|---|---|---|
| 1 quart / 1 liter | 2 drops | 2 drops | 2 drops (0.1 mL) | 10 drops (½ mL) |
| 1 gallon | 8 drops | 6 drops | 8 drops (½ mL; "a little less than ⅛ teaspoon") | 40 drops (2½ mL; ½ tsp) |
| 2 gallons | 16 drops (¼ tsp) | 12 drops (⅛ tsp) | — | — |
| 4 gallons | ⅓ tsp | ¼ tsp | — | — |
| 5 gallons | — | — | 40 drops (2½ mL; ½ tsp) | 200 drops (12½ mL; 2½ tsp) |
| 8 gallons | ⅔ tsp | ½ tsp | — | — |

- CDC [PD-quote]: "If the water is cloudy, murky, colored, or very cold, add double the amount of bleach listed below." Let it stand at least 30 minutes.
- EPA [PD-quote]: "Stir and let stand for 30 minutes. The water should have a slight chlorine odor. If it doesn't, repeat the dosage and let stand for another 15 minute[s]."
- Ready.gov (2021) gives 1/8 teaspoon per gallon and says to use only bleach with "5.25 to 6.0 percent sodium hypochlorite". That range is dated because 8.25% bleach is now common, so the engine should use the concentration-specific table.

**Other methods (CDC 2024, [PD-quote] or close paraphrase)**
- Chlorine dioxide tablets kill *Cryptosporidium* if used correctly. Iodine and chlorine tablets do not.
- Avoid iodine-treated water if pregnant, if you have thyroid problems, or if you are hypersensitive to iodine. Nobody should drink it for more than a few weeks.
- Portable filters: "Will not remove viruses". An absolute pore size of **0.3 micron** or smaller is needed for bacteria and **1 micron** or smaller for parasites. Add a chemical disinfectant after filtering.
- UV only works in clear water. Solar disinfection: clear plastic bottles on their side for **6 hours (sunny) or 2 days (cloudy)**.
- Water contaminated with fuel, toxic chemicals, or radioactive material **cannot** be made safe by boiling or disinfection.
- Ready.gov distillation method: boil for 20 minutes with an inverted cup under the lid.

### 1.5 Water storage and treatment costs (as of 2026-09-25 unless noted)

| Item | Price | $/gal capacity (DERIVED) | Source |
|---|---|---|---|
| Reliance Aqua-Tainer 7 gal (26 L) | $23.99 | 3.43 | https://relianceoutdoors.com/products.json (handle aqua-tainer-4g-15l) |
| Reliance Aqua-Tainer 4 gal | $22.99 | 5.75 | same |
| Reliance Jumbo-Tainer 2.0 (7 gal) | $20.99 | 3.00 | same |
| Reliance Water-Pak 5 gal / Aqua-Pak 5 gal | $23.99 / $22.99 | 4.80 / 4.60 | same |
| Reliance Aqua-Tank 32 / 18 / 8 gal | $137.99 / $110.99 / $84.99 | 4.31 / 6.17 / 10.62 | same |
| Augason Farms 55-gal Water Storage & Treatment Kit (sale; local pickup; list price $309.99) | $129.99 | 2.36 (5.64 at list) | https://www.augasonfarms.com/products.json |
| LifeStraw Personal / Family / Peak Gravity 8 L / Community | $17.95 / $54.95 / $95.95 / $399.00 | — | https://www.lifestraw.com/products.json |
| Aquamira chlorine dioxide treatment (My Patriot Supply) | $9.99 | — | https://www.mypatriotsupply.com/products.json |
| Retail bottled water, per gallon | **UNVERIFIED** (not captured) | — | — |

### 1.6 Suggested water formula (DERIVED)

`water_gal = Σ_persons(days × 1.0 × climate_factor) + Σ_lactating(days × 0.29) + Σ_pregnant(days × 0.08) + Σ_formula_infants(days × 0.25) + Σ_pets(days × weight_lb/128) + Σ_livestock(days × 5.3–7.9) + dehydrated_food_person_days × 0.6`

Here climate_factor = 1.0 (temperate) or 1.75-2.0 (hot). The dog and cat term uses 1 oz/lb/day ÷ 128 oz/gal. Livestock uses 20-30 L per large animal per day ÷ 3.785.

---

## 2. FOOD

### 2.1 Energy requirements

**Sphere 2018 planning figure** [paraphrase] (Food Security Std 6.1 and Appendix; ReliefWeb PDF pp. 198, 231):
- **2,100 kcal per person per day**, with 10-12% of energy from protein and 17% from fat.
- The Appendix table sets protein at 53 g and fat at 40 g.
- This is a *population average* across all ages and both sexes, and it already includes pregnancy and lactation. Sphere says it should not be used as an individual requirement.
- Adjust it upward when activity exceeds light (1.6 × BMR), when mean ambient temperature is **below 20°C**, or when the population is malnourished. Adjust it for demographics and body weights.
- For individual household sizing, use the USDA table below and treat 2,100 as a sanity-check average.

**USDA/HHS *Dietary Guidelines for Americans 2020-2025*, Appendix 2** [PD]. Based on IOM EER equations (2002/2005). Reference man: 5'10", 154 lb. Reference woman: 5'4", 126 lb. The PDF was retrieved via Wayback because the live site blocks fetches: https://web.archive.org/web/20250101015914id_/https://www.dietaryguidelines.gov/sites/default/files/2020-12/Dietary_Guidelines_for_Americans_2020-2025.pdf (pp. 139-141).

Activity definitions [PD-quote]:
- **Sedentary:** "only the physical activity of independent living".
- **Moderately active:** "walking about 1.5 to 3 miles per day at 3 to 4 miles per hour".
- **Active:** "walking more than 3 miles per day at 3 to 4 miles per hour".

Table A2-2, estimated kcal/day, ages 2 and older (Sedentary / Moderately active / Active):

| Age | Male S | Male M | Male A | Female S | Female M | Female A |
|---|---|---|---|---|---|---|
| 2 | 1,000 | 1,000 | 1,000 | 1,000 | 1,000 | 1,000 |
| 3 | 1,000 | 1,400 | 1,400 | 1,000 | 1,200 | 1,400 |
| 4 | 1,200 | 1,400 | 1,600 | 1,200 | 1,400 | 1,400 |
| 5 | 1,200 | 1,400 | 1,600 | 1,200 | 1,400 | 1,600 |
| 6 | 1,400 | 1,600 | 1,800 | 1,200 | 1,400 | 1,600 |
| 7 | 1,400 | 1,600 | 1,800 | 1,200 | 1,600 | 1,800 |
| 8 | 1,400 | 1,600 | 2,000 | 1,400 | 1,600 | 1,800 |
| 9 | 1,600 | 1,800 | 2,000 | 1,400 | 1,600 | 1,800 |
| 10 | 1,600 | 1,800 | 2,200 | 1,400 | 1,800 | 2,000 |
| 11 | 1,800 | 2,000 | 2,200 | 1,600 | 1,800 | 2,000 |
| 12 | 1,800 | 2,200 | 2,400 | 1,600 | 2,000 | 2,200 |
| 13 | 2,000 | 2,200 | 2,600 | 1,600 | 2,000 | 2,200 |
| 14 | 2,000 | 2,400 | 2,800 | 1,800 | 2,000 | 2,400 |
| 15 | 2,200 | 2,600 | 3,000 | 1,800 | 2,000 | 2,400 |
| 16-18 | 2,400 | 2,800 | 3,200 | 1,800 | 2,000 | 2,400 |
| 19-20 | 2,600 | 2,800 | 3,000 | 2,000 | 2,200 | 2,400 |
| 21-25 | 2,400 | 2,800 | 3,000 | 2,000 | 2,200 | 2,400 |
| 26-30 | 2,400 | 2,600 | 3,000 | 1,800 | 2,000 | 2,400 |
| 31-35 | 2,400 | 2,600 | 3,000 | 1,800 | 2,000 | 2,200 |
| 36-40 | 2,400 | 2,600 | 2,800 | 1,800 | 2,000 | 2,200 |
| 41-45 | 2,200 | 2,600 | 2,800 | 1,800 | 2,000 | 2,200 |
| 46-50 | 2,200 | 2,400 | 2,800 | 1,800 | 2,000 | 2,200 |
| 51-55 | 2,200 | 2,400 | 2,800 | 1,600 | 1,800 | 2,200 |
| 56-60 | 2,200 | 2,400 | 2,600 | 1,600 | 1,800 | 2,200 |
| 61-65 | 2,000 | 2,400 | 2,600 | 1,600 | 1,800 | 2,000 |
| 66-75 | 2,000 | 2,200 | 2,600 | 1,600 | 1,800 | 2,000 |
| 76+ | 2,000 | 2,200 | 2,400 | 1,600 | 1,800 | 2,000 |

- **Toddlers (Table A2-1), males/females:** 12 mo 800/800; 15 mo 900/800; 18 mo 1,000/900; 21-23 mo 1,000/1,000.
- **Pregnancy/lactation (Table A2-3), change from pre-pregnancy needs:** 1st trimester +0; 2nd trimester **+340**; 3rd trimester **+452**; lactation months 0-6 **+330**; lactation months 7-12 **+400**.
- **Engine note:** during disasters, activity is usually "moderately active" or higher (hauling water, clean-up, walking). DERIVED suggestion: default adults to the Moderately Active column. Use Sedentary only for shelter-in-place tiers, and let the user override.
- Cold-climate uplift: Sphere says requirements rise below 20°C mean ambient temperature, but gives no increment in the handbook text. A commonly cited increment of +100 kcal per 5°C below 20°C (UNHCR/UNICEF/WFP/WHO 2002) is **UNVERIFIED** here.

### 2.2 How much food authorities tell households to hold

| Authority | Duration | Notes | Source |
|---|---|---|---|
| Ready.gov Food (updated 03/12/2026) | "at least a several-day supply" | [PD-quote] "Store at least a several-day supply of non-perishable food." Foods listed include ready-to-eat canned meats/fruits/vegetables with a can opener, protein or fruit bars, dry cereal or granola, peanut butter, dried fruit, canned juices, non-perishable pasteurized milk, high-energy foods, food for infants, and comfort foods. | https://www.ready.gov/food |
| Ready.gov Food, outage food safety | — | [PD-quote] "The refrigerator will keep food cold for about four hours if it is unopened." / discard perishables "above 40 degrees Fahrenheit for two hours or more" / "Twenty-five pounds of dry ice will keep a 10 cubic foot freezer below freezing for three to four days." Ready.gov Power Outages adds [PD-quote] "A full freezer will keep the temperature for about 48 hours." | https://www.ready.gov/food ; https://www.ready.gov/power-outages |
| Red Cross | 3 days (evacuation), 2 weeks (home) | [paraphrase] | Red Cross Survival Kit Supplies (Wayback link in §1.1) |
| CDC (pregnancy/postpartum page) | at least 3 days of food and water | [PD-quote] "at least a 3-day supply of food and water for each person" | https://www.cdc.gov/reproductive-health/emergency-preparation-response/safety-messages.html |
| Oregon OEM / Washington EMD | 2 weeks | [paraphrase] | §11 |
| Church of Jesus Christ | 3-month supply of everyday foods, plus a longer-term supply | [paraphrase] | §2.3 |
| Germany BBK | 10 days (at least 3) | [paraphrase] | §1.1 |

FEMA's classic booklet *Food and Water in an Emergency* (FEMA L-210 / ARC 5055) could not be retrieved in this pass. Any figure attributed to it is **UNVERIFIED**.

### 2.3 Church of Jesus Christ of Latter-day Saints long-term food storage: actual figures

**(a) Current official policy (since 2007).** The pamphlet *All Is Safely Gathered In: Family Home Storage* (© 2007, item 04008) [paraphrase] names four elements:
1. A **three-month supply** of foods that are part of your normal diet. Start with a one-week supply and build up to three months, rotating regularly.
2. **Drinking water.**
3. A **financial reserve**, built by saving a little each week.
4. A **longer-term supply**, where permitted, of foods that keep for a long time such as **wheat, white rice, and beans**. These keep 30+ years when properly packaged and stored cool and dry.

The pamphlet gives **no per-person quantities**, warns against going into debt, and says to store as much as circumstances allow.
https://www.churchofjesuschrist.org/bc/content/shared/content/english/pdf/language-materials/04008_eng.pdf

The current *Emergency Preparedness* manual (2023) [paraphrase] repeats this. For short-term storage, build 1 day, then 1 week, then 1 month and beyond. For long-term quantities it points readers to the BYU guide in (c). https://www.churchofjesuschrist.org/study/manual/emergency-preparedness/01-food-and-water-storage/01-preparing-emergency-food-storage?lang=eng

**(b) The classic "year's supply of basics" figures.** Source: *Ensign* (official Church magazine), March 2006, "Food Storage for One Year", citing the First Presidency letter of Jan. 20, 2002 [numbers; paraphrase]. https://www.churchofjesuschrist.org/study/ensign/2006/03/random-sampler/food-storage-for-one-year?lang=eng

| Item (one adult, one year) | Quantity |
|---|---|
| Grains (wheat, flour, rice, corn, oatmeal, pasta) | **400 lb (181 kg)** |
| Legumes (dry beans, split peas, lentils) | **60 lb (27 kg)** |
| Powdered milk | **16 lb (7 kg)** |
| Cooking oil | **10 qt (9 L)** |
| Sugar or honey | **60 lb (27 kg)** |
| Salt | **8 lb (3.6 kg)** |
| Water (a two-week reserve, not a year) | **14 gal (53 L)** |

Child portions as a share of the adult portion:

| Child age | Share of adult portion |
|---|---|
| 3 and under | 50% |
| 4-6 | 70% |
| 7-10 | 90% |
| 11 and up | 100% |

The article adds [paraphrase]:
- Add one year to each child's current age when calculating, because children are still growing.
- Nursing infants share their mother's portion.
- Young children and pregnant or nursing women need more milk.

Corroboration: Deseret News, Feb 20, 2002, gives the same figures (https://www.deseret.com/2002/2/20/19638826/lds-stress-storage-savings/). USU Extension's *Food Storage* booklet (©2013, citing Eliason & Lloyd 2005) also repeats them. USU notes [paraphrase] that this supply covers calories for one year but may lack calcium and vitamins A, C, B12 and E, and that a family of four's long-term supply can weigh **1,500-2,000 lb**. https://extension.usu.edu/preserve-the-harvest/files/Food-Storage-Booklet.pdf

**(c) The current Church-linked quantity guide.** BYU Dept. of Nutrition, Dietetics & Food Science, *An Approach to Longer-Term Food Storage*, revised Sept 2019. The Church manual links it as its food-storage calculator guide. It gives adequate calories and protein for one adult for one year [numbers; paraphrase]. https://brightspotcdn.byu.edu/b1/4d/75fc449e4ce9843daa701f69faa4/an-approach-to-longer-term-food-storage.SEPT2019.pdf

| Long-term item (≥30-yr shelf life unless noted) | Per adult per year | Approx. lb |
|---|---|---|
| Wheat | 24 #10 cans | 132 |
| White rice | 12 cans | 65 |
| Rolled oats | 12 cans | 29 |
| Pasta | 6 cans | 21 |
| Legumes | 12 cans | 62 |
| Nonfat dry milk (15-yr) | 12 cans (or 28 pouches) | 49 |
| Sugar (or other sweeteners) | 12 cans | 70 |
| Dried apple slices | 6 cans | 6 |
| Potato flakes | 12 cans | 22 |
| Dried carrots (10-yr) | 3 cans | 8 |
| Dried onions | 1 can | 2 |
| Salt, iodized | 8 lb | 8 |
| Baking soda / baking powder | 1 lb / 4 lb | 5 |
| Vitamin C tablets (90 mg) | 365 tablets | — |

| Short-term item (rotate) | Per adult per year |
|---|---|
| Cooking/salad oils | 2 gal |
| Shortening | 3 cans × 3 lb |
| Butter or margarine (frozen) | 6 lb |
| Mayonnaise or dressings | 3 qt |
| Peanut butter | 6 lb |
| Fruit drink mix | 3 cans |
| Dried eggs | 2 cans |
| Yeast | 2 lb |

Water under this guide is 14 gal per person (two weeks), plus a means of purification.

DERIVED weights:
- BYU 2019 grains: 132 + 65 + 29 + 21 = **247 lb**.
- BYU 2019 long-term dry items total: ≈ **479 lb/adult/yr**.
- 2006 basics: ≈ **563 lb/adult/yr**, excluding water. This assumes oil at about 0.92 kg/L, which is an UNVERIFIED density.

### 2.4 Shelf lives

**USDA FSIS, *Shelf-Stable Food Safety*** [PD]. Retrieved from the Wayback capture of https://www.fsis.usda.gov/food-safety/safe-food-handling-and-preparation/food-safety-basics/shelf-stable-food

| Product (unopened, pantry) | Storage time |
|---|---|
| Low-acid canned goods (meat, poultry, stews, soups other than tomato, pasta products, potatoes, corn, carrots, spinach, beans, beets, peas, pumpkin) | **2-5 years** |
| High-acid canned goods (tomato and fruit juices, tomatoes, fruits, pickles, sauerkraut, vinegar-based foods) | **12-18 months** |
| Home-canned foods | 12 months. FSIS says to boil 10 minutes (high-acid) or 20 minutes (low-acid) before use. |
| Jerky, commercially packaged / home-dried | 12 months / 1-2 months |
| Hard or dry sausage | 6 weeks in the pantry |
| MREs | [PD-quote] "if stored at 120 degrees F ... the MRE should be used within a month. Stored at 60 degrees F, an MRE can last 7 years or more." |
| Storage temperature | [PD-quote] "Temperatures below 85 degrees F are best." Temperatures over 100°F are harmful. Discard bulging, leaking, heavily rusted, or deeply dented cans, or any dent on a seam. |
| Dates | Except for infant formula (and some baby food), product dates indicate quality, not safety. |

**Church life-sustaining shelf-life estimates** (BYU studies; room temperature or below, ≤75°F/24°C; about 10% moisture or less; low-oxygen packaging) [numbers]:

| Food | Years |
|---|---|
| Wheat | 30+ |
| White rice | 30+ |
| Corn | 30+ |
| Sugar | Indefinite |
| Pinto beans | 30 |
| Rolled oats | 30 |
| Pasta | 30 |
| Potato flakes | 30 |
| Apple slices | 30 |
| Nonfat powdered milk | 20 |
| Dehydrated carrots | 10 |

Not recommended for long-term storage [paraphrase]:
- pearled barley, jerky, dried eggs, nuts
- whole-wheat flour, brown rice, milled grains other than rolled oats
- brown sugar, granola, chocolate
- dehydrated produce that is not snap-dry
- bottled or canned butter

**Botulism warning:** moist foods must never go into oxygen-reduced packaging.
https://www.churchofjesuschrist.org/study/manual/emergency-preparedness/01-food-and-water-storage/03-long-term-food-and-water-storage?lang=eng

**Home Storage Center order form (U.S., prices effective Jan 1, 2026)** lists these storage lives in years [numbers]:

| Product | Years |
|---|---|
| Apple slices | 30 |
| Beans (black, pinto, great northern) | 25 |
| Carrots | 10 |
| Hot cocoa | 2 |
| White flour | 10 |
| Macaroni | 25 |
| Nonfat dry milk (pouch) | 15 |
| Oats (quick and regular) | 20 |
| Dry onions | 30 |
| Pancake mix | 2 |
| Potato flakes | 20 |
| White rice | 30 |
| Freeze-dried strawberries | 30 |
| Spaghetti bites | 25 |
| Sugar | 30 |
| Wheat | 30 |

The order form footnotes that these lives assume proper packaging and dry storage below 75°F. Several values are shorter than the Church web page's estimates; see §13.
https://assets.churchofjesuschrist.org/01/b4/01b41d0ffdbf11ebbb5eeeeeac1e9b2afdc29be5/food_storage_center_products.pdf

**Temperature effect** (USU Extension booklet, citing BYU, Green et al. 2005) [paraphrase]: wheat kept acceptable quality for **25 years stored cool** (basement) but **only 5 years stored hot** (garage or attic). The optimal range is about **40-70°F**. Plastic containers let significant oxygen into oils within 1-2 years.

**Engine note:** apply a temperature penalty. If the user stores food in a garage or attic, cut long-term shelf-life assumptions (e.g., 30 → 5 years for grains, per USU/BYU).

### 2.5 Infant formula and special diets

**Feeding guidance.** CDC, *Feeding Your Child Safely During a Disaster* (Feb 20, 2025) [PD]:
- Breastfeeding remains the best option.
- If formula-feeding, [PD-quote] "provide ready-to-use infant formula if available."
- Otherwise use bottled water for powdered or concentrated formula, or boil water for 1 minute and cool it. Use treated water only if bottled or boiled water is unavailable.
- Take extra precautions against *Cronobacter* for infants under 2 months, preterm infants, and immunocompromised infants.

https://www.cdc.gov/breastfeeding/php/guidelines-recommendations/feeding-your-child-safely-during-a-disaster.html

**Kit contents.** CDC *Emergency List for Families With Infants and Young Children* (Aug 3, 2026) [PD]:
- One or two boxes of disposable nursing pads.
- A manual pump.
- Ready-to-feed single-serving formula, plus powdered formula and preparation materials.
- Bottled water for mixing.
- [PD-quote] "check your emergency kit monthly to be sure you have enough formula to meet your baby's current needs for several days."
- Consider a small camp stove, fuel and a pot for boiling water.

https://www.cdc.gov/infant-feeding-emergencies-toolkit/php/checklist.html

**Volumes.** AAP (HealthyChildren.org) [numbers]:
- On average **2½ oz (75 mL) of formula per lb of body weight per day**.
- Usually **no more than 32 oz (960 mL) per 24 h**.
- By 6 months, 6-8 oz at each of 4-5 feedings.

https://www.healthychildren.org/English/ages-stages/baby/formula-feeding/Pages/Amount-and-Schedule-of-Formula-Feedings.aspx

DERIVED formula per 30 days:

| Infant weight | Prepared formula per 30 days |
|---|---|
| 7 lb | 525 oz |
| 10 lb | 750 oz |
| 12 lb | 900 oz |
| ≥12.8 lb | 960 oz (capped at 32 oz/day) |

**Special diets and conditions:**
- Ready.gov [PD-quote]: "Remember any special dietary needs."
- CDC *Diabetes Care During Emergencies*: pack diabetes supplies for **at least 1-2 weeks** (§3.2). The same page links a 3-day emergency diet for people on dialysis. https://www.cdc.gov/diabetes/articles/diabetes-care-emergencies.html
- Allergens: Home Storage Center products are packed in a facility that also processes wheat, milk, soy, almonds and coconut.

### 2.6 Cooking fuel

**Energy content** (EIA, "British thermal units", [PD]):

| Fuel | Energy content |
|---|---|
| Propane | **91,452 Btu/gal** |
| Motor gasoline | 120,214 Btu/gal |
| Diesel | 137,381 Btu/gal |
| Heating oil | 138,500 Btu/gal |
| Natural gas | 1,036 Btu/ft³ |
| Wood | 20,000,000 Btu/cord |
| Electricity | 1 kWh = 3,412 Btu |

https://www.eia.gov/energyexplained/units-and-calculators/british-thermal-units.php

**Safety rules:**
- CDC [PD-quote]: "Never use a portable gas camp stove indoors." / "Never burn charcoal indoors." / "Never heat your house with a gas oven." (§5.1)
- Home fuel storage limits are local fire-code matters (§6.3).

**DERIVED fuel for boiling water (planning estimate only).** Heating 1 L from 20°C to 100°C takes 335 kJ, about 318 Btu (thermodynamic). At an assumed 40% stove efficiency (UNVERIFIED) that is about 800 Btu per liter boiled, or about 0.009 gal of propane per liter. At an assumed 4.2 lb/gal propane density (UNVERIFIED), 1 lb of propane boils about 25-30 L. Validate against stove specs before shipping.

**Cooking time:** Sphere [paraphrase] advises choosing foods that need little cooking when fuel is scarce. Canned and ready-to-eat foods need zero fuel.

### 2.7 Cost per person-day by approach

| Approach | Cost basis | $/person-day (DERIVED) | kcal/day delivered | Source |
|---|---|---|---|---|
| Everyday groceries, **USDA Thrifty Food Plan**, Aug 2026 (issued Sept 2026) | Reference family of 4: **$236.30/week**, $1,023.70/month | **$8.44** | nutritionally complete | https://www.fns.usda.gov/sites/default/files/resource-files/CostofFoodAug2026Thrifty.pdf |
| TFP by person | Male 20-50: $73.60/wk; female 20-50: $58.50; child 1 yr: $26.40; 6-8 y: $48.30; 9-11 y: $55.80; male 71+: $61.90; female 71+: $60.10 | $10.51; $8.36; $3.77; $6.90; $7.97; $8.84; $8.59 | — | same |
| TFP household-size adjustment | 1-person +20%; 2-person +10%; 3-person +5%; 4-person 0; 5-6 persons −5%; 7+ persons −10% | — | — | same |
| USDA Low / Moderate / Liberal plans (Aug 2026) | Male 19-50: $73.20 / $91.90 / $112.20 per week. Female 19-50: $63.60 / $77.60 / $99.00 per week. | M $10.46 / $13.13 / $16.03; F $9.09 / $11.09 / $14.14 | — | https://www.fns.usda.gov/sites/default/files/resource-files/CostofFoodAug2026LowModLib.pdf |
| **Shelf-stable pantry (canned, dry goods) = 2-week or 3-month tier** | Assumption: cost ≈ Thrifty-to-Low plan levels | ≈ $8.4-10.5 (UNVERIFIED proxy) | depends | — |
| **Church bulk basics (2006 figures) at Home Storage Center prices** | 400 lb wheat at $7.00/5.5-lb can ($1.273/lb); 60 lb pinto beans at $8.83/5.2 lb; 16 lb nonfat dry milk at $6.92/27 oz; 60 lb sugar at $10.00/5.6 lb. Total **$783.73/yr**; oil and salt not priced (UNVERIFIED). | **$2.15** ($2.51 if rice replaces wheat) | ≈2,100+ (USU says calories are adequate; not computed here) | US order form (Jan 1, 2026) above. The Church online store sells a case of 6 wheat cans for $42, confirming $7.00 per can: https://store.churchofjesuschrist.org/usa/en/hard-red-wheat---case-of-6-cans/5638679053.p |
| **BYU 2019 list at Home Storage Center prices** | Long-term items (wheat, rice, oats, macaroni, pinto beans, 28 dry-milk pouches, sugar, apples, potato flakes, carrots, onions) = **$1,040.34/yr**. Salt, leavening, vitamin C and short-term items (oils, peanut butter) not included. | **$2.85** + short-term items | "adequate calories" per BYU | same |
| Freeze-dried entrées: Mountain House 3-day / 14-day / 30-day / 3-month / 1-year | $99.99 / $457.99 / $939.99 / $2,749.99 / $10,366.99 | $33.33 / $32.71 / $31.33 / $30.56 / $28.40 | ~1,706-1,732 (below the 2,100 planning figure) | https://mountainhouse.com/products.json (2026-09-25) |
| Mountain House normalized cost | — | **$32.80-$39.07 per 2,000 kcal** | — | DERIVED |
| Dehydrated "bucket" kits: Augason Farms 72-h / 30-day / 3-month / 1-year | $33.99 / $182.99 / $359.99 / $1,383.99 | $11.33 / $6.10 / $4.00 / $3.79 | 1,580 / **1,290** / 1,290 / 1,290 | https://www.augasonfarms.com/products.json |
| Augason normalized cost | — | $5.88-$14.34 per 2,000 kcal | — | DERIVED |
| Ready Hour (My Patriot Supply) 1-week / 4-week / 3-month / 6-month | $89.95 / $277.95 / $797.95 / $1,595.90 | $12.85 / $9.93 / $8.87 / $8.87 | "2,000+" per vendor | https://www.mypatriotsupply.com/products.json |
| ReadyWise 72-Hour Kit Bundle (4-pack) | $179.96 (21,840 kcal) | $15.00 | 1,820 | https://www.readywise.com/products.json |

**Key point for the engine.** Every pre-made kit is sized by vendor "servings" or "days". Normalize them to kcal: `kit_person_days = total_kcal / required_kcal_per_person_day`. Many "30-day" kits supply only 1,290-1,730 kcal/day, which is 60-80% of an adult's needs. That makes a "30-day" kit roughly an 18-25 day kit for an active adult.

---

## 3. MEDICAL

### 3.1 First-aid kit (household of four)

American Red Cross, "Anatomy of a First Aid Kit" (Wayback capture 2025-12-31). Item list presented as data.
https://web.archive.org/web/20251231180252/https://www.redcross.org/get-help/how-to-prepare-for-emergencies/anatomy-of-a-first-aid-kit.html

| Qty | Item |
|---|---|
| 2 | absorbent compress dressings, 5 × 9 in |
| 25 | adhesive bandages, assorted sizes |
| 1 | adhesive cloth tape, 10 yd × 1 in |
| 5 | antibiotic ointment packets, about 1 g each |
| 5 | antiseptic wipe packets |
| 2 | packets of aspirin, 81 mg each |
| 1 | emergency blanket |
| 1 | breathing barrier with one-way valve |
| 1 | instant cold compress |
| 2 pair | nonlatex gloves, size large |
| 2 | hydrocortisone ointment packets, about 1 g each |
| 1 | gauze roll (roller) bandage, 3 in |
| 1 | roller bandage, 4 in wide |
| 5 | sterile gauze pads, 3 × 3 in |
| 5 | sterile gauze pads, 4 × 4 in |
| 1 | oral thermometer, non-mercury/non-glass |
| 2 | triangular bandages |
| 1 | tweezers |
| 1 | emergency first-aid instructions |

The Red Cross also advises [paraphrase]:
- Add personal medications and emergency phone numbers.
- Check the kit regularly.
- Replace used or expired items.

For scaling, treat the list as sized for 4 people. DERIVED: scale consumables (bandages, wipes, gauze, gloves) linearly per extra person, and keep durable items (tweezers, thermometer, breathing barrier) at one per kit.

**Prices** (Red Cross Store, Wayback 2026-08-30): Deluxe Family First Aid Kit, 115-piece **$49.95**; Deluxe All Purpose First Aid Kit, 299-piece **$34.95**; Be Red Cross Ready First Aid Kit $30.00; First Aid Kit PLUS $33.85; Deluxe 137-piece Auto Kit $31.75. https://web.archive.org/web/20260830185522/https://www.redcross.org/store/first-aid-supplies/first-aid-kits
- Premium trauma-oriented example: My Medic MyFAK, 111 pieces, **$169.95** (My Patriot Supply products.json).

### 3.2 Prescription medication supply

| Authority | Recommended on-hand supply | Wording / paraphrase | Source |
|---|---|---|---|
| CDC (pregnant, postpartum and breastfeeding page) | **7-10 days on hand**, plus ask about a **30-day or longer** emergency refill | [PD-quote] "Have at least a 7- to 10-day supply of your prescription medications. Ask your health care provider if you can obtain a 30-day (or longer) emergency prescription refill. Learn more about emergency prescription laws in your state." | https://www.cdc.gov/reproductive-health/emergency-preparation-response/safety-messages.html |
| CDC, *Diabetes Care During Emergencies* | **at least 1-2 weeks** of diabetes supplies | [PD-quote] "Pack enough diabetes supplies to last at least 1 to 2 weeks". The kit covers insulin and syringes for every injection, oral medicines, glucose meter, extra batteries, lancets, pump supplies, glucagon, ketone strips, alcohol wipes, glucose tablets (15 g quick carbs), and a sharps container. | https://www.cdc.gov/diabetes/articles/diabetes-care-emergencies.html |
| American Red Cross | **7-day** supply | [paraphrase] A 7-day medication supply plus medical items. | Red Cross Survival Kit Supplies (Wayback link in §1.1) |
| Florida Division of Emergency Management | **minimum two-week supply** | [paraphrase] | https://www.floridadisaster.org/planprepare/disability/personal-and-family-plans/medication/ |
| Ready.gov (Disability page, 09/09/2026) | no number given | [PD-quote] "Talk to your doctor or pharmacist about how you can create an emergency supply of medicines." Keep a list of medicines with diagnosis, dosage, frequency, supply needs and allergies. | https://www.ready.gov/disability |
| ASPCA (pets) | **2-week** supply of pet medicine | [paraphrase] | https://www.aspca.org/pet-care/general-pet-care/disaster-preparedness |

**90-day supplies.**
- No CDC, FEMA or FDA document recommending a *90-day household stock* for disaster preparedness was located (UNVERIFIED).
- 90-day maintenance fills are a pharmacy-benefit feature; check the individual plan.
- North Carolina law allows up to 90 days as an *emergency* refill (see below).

**State emergency-refill laws.** Healthcare Ready, *A Review of State Emergency Prescription Protocols* (published Sept 18, 2022, modified Nov 8, 2023) [paraphrase; counts may be dated]:
- **12 of 51 jurisdictions** (50 states + DC) have emergency-refill rules tied to public-health emergencies.
- **10 states** activate on a governor's emergency declaration and allow **up to 30 days** or a reasonable quantity.
- **North Carolina** allows up to **90 days** after an interruption of medical services.
- **New Hampshire** allows **72 hours**.
- **23 jurisdictions** have general (non-disaster) emergency-refill rules: 15 of them allow only **72 hours**, 3 allow 7-10 days, and 5 leave the quantity to the pharmacist.
- **16 jurisdictions** have no emergency-refill law at all.

https://healthcareready.org/a-review-of-state-emergency-prescription-protocols/

State examples:
- **Florida** [paraphrase]: a 30-day refill, even if you just refilled, **only** if your county is under an NWS hurricane warning, a governor's emergency order, or an activated emergency operations center. Insurers must waive "refill too soon" limits in those cases.
- Georgia and Texas 30-day provisions and the "Kevin's Law" state list (Ohio 2015, 30 days for non-controlled drugs, later copied by other states) come only from search-result summaries of Pharmacy Times, the Georgia Board of Pharmacy and the Texas Board of Pharmacy. They are **UNVERIFIED at primary source**.

**Uninsured people.** HHS/ASPR's **Emergency Prescription Assistance Program (EPAP)** helps uninsured people replace prescriptions, some medical supplies and equipment, and get vaccines, but only in areas where EPAP has been activated after a declared disaster. https://aspr.hhs.gov/EPAP/Pages/default.aspx

### 3.3 Medication storage and expiry

- **Refrigerated medicines.** Ready.gov (Power Outages) [PD-quote]: "If the power is out for more than a day, discard any medication that should be refrigerated, unless the drug's label says otherwise." Ask your provider how long each medicine can tolerate higher temperatures.
- **Heat.** CDC *About Heat and Your Health* [PD-quote]: "Have a plan for what to do with refrigerated medications and electronic medical devices."
- **Insulin.** CDC *Managing Insulin in an Emergency* [PD]:
  - Keep it cool but never frozen, out of direct heat and sun.
  - If it was stored above 86°F, monitor blood sugar.
  - Afterwards, discard insulin that was stored at room temperature or extreme temperatures.
  - https://www.cdc.gov/diabetes/articles/managing-insulin-in-emergency.html
- **Expired medicines.** FDA, *Don't Be Tempted to Use Expired Medicines* [PD-quote]: "sub-potent antibiotics can fail to treat infections, leading to more serious illnesses and antibiotic resistance." Keep medicines away from hot appliances and the sink. https://www.fda.gov/drugs/special-features/dont-be-tempted-use-expired-medicines
- **Shelf-life extension does not apply to households.** FDA's Shelf-Life Extension Program (SLEP) [PD] is **limited to federal stockpiles**. It is run by DoD, and participants are federal agencies. Its results are not a license for households to use expired drugs. https://www.fda.gov/emergency-preparedness-and-response/mcm-legal-regulatory-and-policy-framework/expiration-dating-extension

### 3.4 ANTIBIOTICS: what reputable sources actually say

| Source | Position | Link |
|---|---|---|
| **CDC, Be Antibiotics Aware** (Sept 23, 2025) | [PD-quote] "Do not share your antibiotics with others." / "Do not save them for later. Taking the wrong medicine for a future illness may delay correct treatment and can cause severe side effects." / "Do not take antibiotics prescribed for someone else." CDC also says [PD-quote] "In children, side effects from antibiotics are the most common cause of medication-related emergency department visits." | https://www.cdc.gov/antibiotic-use/about/index.html |
| **CDC Yellow Book 2026**, *Travel Health Kits* (Backer & Smith) | [paraphrase] Carrying some prescribed drugs for presumptive treatment in certain regions is common practice, e.g., for travelers' diarrhea, malaria or altitude illness. The personal kit lists an antibiotic for travelers' diarrhea when one has been prescribed. An *expanded* kit for groups going to remote areas (possibly carried by a licensed clinician) suggests options by organ system: azithromycin or ciprofloxacin for travelers' diarrhea; nitrofurantoin, ciprofloxacin or TMP-SMX for UTI; azithromycin, amoxicillin or amoxicillin-clavulanate for respiratory infection; cephalexin for skin infection. **This is the closest official precedent for "standby" antibiotics, but it is framed as prescribed, remote-travel, clinician-guided use.** | https://www.cdc.gov/yellow-book/hcp/preparing-international-travelers/travel-health-kits.html |
| **CDC Yellow Book 2026**, *Travelers' Diarrhea* (Connor & Leung) | [paraphrase] Prophylactic antibiotics are **not recommended for most travelers**. The risks (side effects, *C. difficile*, and carriage of resistant bacteria such as ESBL-producing Enterobacteriaceae) outweigh the benefits. Standby self-treatment is reserved for moderate or severe illness. Fluoroquinolone resistance and FDA boxed warnings limit ciprofloxacin. Oral rehydration is the cornerstone of treatment. | https://www.cdc.gov/yellow-book/hcp/preparing-international-travelers/travelers-diarrhea.html |
| **Wilderness Medical Society**, wound-management guideline (2014 update, *Wilderness Environ Med*) | [paraphrase] With a few exceptions, there is scant evidence for routine prophylactic antibiotics in wounds. Systemic antibiotics **are** indicated for **open fractures (grade 1A), human bites (1B) and mammalian bites to the hand (1B)**, and not for burns. Evacuate complex wounds, open fractures, deep-structure injuries, mammalian bites, and wounds showing early infection. The infection risk even with good wound care is 1-12%. Per a 2026 review, WMS kit guidance directs clinicians to choose antibiotics with a travel-medicine physician. A newer WMS wound edition may exist (UNVERIFIED). | https://doi.org/10.1016/j.wem.2014.08.015 (PDF via https://web.archive.org/web/20200709231429id_/https://www.wildmedcenter.com/uploads/5/9/8/2/5982510/wms_wound_care_12-2014.pdf) |
| **FDA on "fish antibiotics"** | FDA warning letter to Chewy, Inc. (Nov 30, 2023) [PD] says aquarium and bird products containing **amoxicillin, cephalexin, ciprofloxacin, doxycycline, metronidazole and azithromycin** are **unapproved and misbranded new animal drugs**, deemed [PD-quote] "unsafe and adulterated". FDA [PD-quote]: "FDA is particularly concerned ... they contain antimicrobials that are considered medically important in the treatment of human disease." FDA's 2018 statement "Ornamental Fish Drugs and You" (cited in the PLoS One paper below) warned that ornamental-fish antibiotics are not FDA-approved, conditionally approved, or indexed. | https://www.fda.gov/inspections-compliance-enforcement-and-criminal-investigations/warning-letters/chewy-inc-664707-11302023 |
| **Evidence of misuse**: Bishop et al., *PLoS One* 2020;15(9):e0238538 | [paraphrase] 24 online vendors sold fish antibiotics for $8.99-$119.99. 2.4% of 2,288 reviews suggested human use, but those reviews drew 30% of all "likes". The pills physically matched FDA-approved human products, but their contents were not verified. | https://pmc.ncbi.nlm.nih.gov/articles/PMC7470343/ |
| **Direct-to-consumer telehealth kits**: Jase Medical ("JaseCase") | [paraphrase] An online intake and evaluation by a board-certified physician (synchronous consult where state law requires), then dispensing by a licensed pharmacy. The case is advertised as 10 medicines: 5 antibiotics (amoxicillin-clavulanate, azithromycin, ciprofloxacin, metronidazole, and one more marketed for Lyme or skin infection) plus 5 symptom drugs. Limit: one per order. The company says medicines are only for when medical help cannot be accessed and that it does not condone stockpiling past expiry. It also sells chronic-medication backup supplies ("Jase Daily") and compounded ivermectin "parasite" kits, which are unavailable in some states. | https://jasemedical.com/ |
| **Duration Health** | [paraphrase; search-result summary only, because the domain would not resolve from this environment] Clinician-led telehealth visit with ID verification. Custom prescription kits drawn from 70+ medications and shipped by a partner mail-order pharmacy. No controlled substances. Items labeled with a 1-year expiry. **UNVERIFIED at primary source.** | https://durationhealth.com/kit/1/about |
| **Critique**: "A Review of Direct-to-Consumer Home Antibiotic Kits: A Threat to Antimicrobial Stewardship and Patient Safety", *Missouri Medicine* 2026 Jul-Aug;123(4):344-350 | [paraphrase] Kits typically cost **$300-400**. The model starts from a pre-defined prescription instead of a diagnosis. It encourages use of a paid-for product, wrong drug/dose/duration, *C. difficile*, and resistance. It notes expired tetracyclines have been linked to kidney injury. It calls the model contrary to CDC and IDSA stewardship principles. Its harm-reduction alternative is on-demand telehealth with an established clinician, with pre-filled prescriptions limited to specific high-risk scenarios such as travelers' diarrhea. | https://pmc.ncbi.nlm.nih.gov/articles/PMC13585011/ |
| **IDSA** | No IDSA statement specifically on household antibiotic stockpiles was located in this pass (**UNVERIFIED**). IDSA's position is inferred only from general stewardship principles, as characterized in the review above. | — |
| **Federal law** | 21 U.S.C. §353(b)(1) [PD-quote]: a drug that "is not safe for use except under the supervision of a practitioner licensed by law ... shall be dispensed only (i) upon a written prescription of a practitioner licensed by law". Systemic antibiotics for humans are prescription-only in the U.S. | https://www.law.cornell.edu/uscode/text/21/353 |

**Recommended position for the app** (my synthesis as a responsible emergency-management default; not a clinical guideline):

1. **Do not compute an antibiotic quantity. Show no dosing.** Render antibiotics as a "Talk to your clinician" card with a default quantity of 0. A household planner that sizes antibiotics per person-day would be practicing medicine and contradicting CDC's "do not save them for later" guidance.
2. **Put the effort into what saves lives in real disasters:**
   - Uninterrupted chronic medications: 7-14 days on hand (CDC 7-10 days, CDC diabetes 1-2 weeks, Florida 2 weeks), plus knowing your state's 30-day emergency-refill rules and EPAP.
   - A medication list, and a cold-chain plan for refrigerated drugs.
   - Wound care: first-aid kit, clean irrigation water, hygiene.
   - ORS.
   - Pre-planned access to care: telehealth account, pharmacy-status tools, 911.
3. **Legitimate exceptions exist, but only through the patient's own clinician** (or a licensed telehealth prescriber) for a specific, foreseeable scenario. Examples: remote or austere travel (CDC Yellow Book model); a known recurrent condition with a written action plan. The app can prompt: "Ask your clinician whether a standby prescription and written instructions make sense for [trip or condition]."
4. **Hard lines the app should state plainly:**
   - Never use aquarium or veterinary antibiotics. FDA calls them unapproved and adulterated, with unverified contents and doses.
   - Never share prescriptions (CDC). Sharing may also violate state pharmacy law; UNVERIFIED per state.
   - Do not self-start antibiotics when care is reachable.
   - Do not use expired antibiotics (FDA).
   - Store per label.
5. **On DTC "emergency antibiotic kits":** describe them neutrally. They are legal when a licensed prescriber issues the prescription under state telehealth rules, cost about $300-400, and are criticized by infectious-disease and stewardship commentators. Do not recommend or link a vendor.

### 3.5 Over-the-counter list

- **Ready.gov (Build a Kit)** [PD-quote]: "Non-prescription medications such as pain relievers, anti-diarrhea medication, antacids or laxatives"; also "Prescription eyeglasses and contact lens solution"; "Soap, hand sanitizer and disinfecting wipes to disinfect surfaces".
- **CDC Yellow Book 2026 personal kit** [paraphrase]: antihistamines; aspirin, acetaminophen or ibuprofen; loperamide; oral electrolyte (ORS) powder; hydrocortisone cream; antifungal cream; antiseptic wipes; antimicrobial ointment; cough drops; decongestant (pseudoephedrine, not phenylephrine); mild laxative; antacid or acid blocker; motion-sickness medicine; temporary dental filling; zinc oxide or sunscreen; permethrin (lice/scabies); elastic bandage; blister pads; wound-closure strips.
- **CDC infant checklist:** infant pain reliever with acetaminophen, infant thermometer, bulb syringe, diaper rash cream.
- Quantities: no authority specifies OTC doses per person-day. DERIVED: one retail package of each per household per tier up to 2 weeks, then scale with duration and household size (assumption).

### 3.6 Masks and respirators

- **CDC, *Masks and Respiratory Viruses Prevention*** (Aug 18, 2025) [PD-quote]: "Cloth masks generally offer lower levels of protection to wearers, surgical/disposable masks usually offer more protection, international filtering facepiece respirators (like KN95 respirators) offer even more, and the most protective respirators are NIOSH Approved® filtering facepiece respirators (like N95® respirators)." It also says to choose the most protective type you can and to check the fit. https://www.cdc.gov/respiratory-viruses/prevention/masks.html
- **Ready.gov kit** [PD-quote]: "Dust mask (to help filter contaminated air)".
- **Ready.gov pandemic page** [PD-quote]: "Gather supplies in case you need to stay home for several days or weeks." and "Buy supplies slowly". It gives no quantities. https://www.ready.gov/pandemic
- No household N95 count is published by CDC or FEMA (searched). DERIVED: `N95 = persons × anticipated exposure-days × 1`, as a user-adjustable assumption. For wildfire smoke, also consider indoor air cleaning (not sized here).

### 3.7 Thermometer

- One oral thermometer (non-mercury, non-glass) is in the Red Cross family kit.
- Add an infant thermometer for households with infants (CDC checklist).
- Hypothermia threshold: CDC [PD-quote] "If a person's temperature is below 95 degrees get medical attention immediately."

### 3.8 Oral rehydration

- **CDC, *Treating Cholera*** (May 29, 2025) [PD]: rehydration, including ORS, is the most important treatment. ORS is made from a prepackaged powder mixed with boiled or treated water. With timely rehydration, over 99% of cholera patients survive. https://www.cdc.gov/cholera/treatment/index.html
- **CDC Yellow Book** [paraphrase]: one ORS packet goes into the indicated volume of safe water, generally **1 L**.
- A home-made *salt solution for heat exhaustion* (not diarrhea): ¼-½ tsp table salt per 1 L of water, optionally with a few teaspoons of sugar or citrus juice for taste (Yellow Book heat chapter).
- The classic home sugar-salt ORS recipe (6 tsp sugar + ½ tsp salt per liter) is **UNVERIFIED** in this pass. Recommend packaged ORS.
- DERIVED stock: ORS packets = persons × 3 packets per tier (for one diarrheal episode each). This is an assumption with no authority figure.

---

## 4. SANITATION AND HYGIENE

### 4.1 Twin-bucket emergency toilet

**Where the guidance comes from.** The Regional Disaster Preparedness Organization of the Portland Metro Region (RDPO) publishes the *Emergency Toilet Guidebook* [paraphrase]. RDPO says the method was developed in Christchurch, New Zealand, after the February 2011 earthquakes. RDPO's task force published a planning basis in the *Journal of Environmental Health* (2019), "Recommendations for Catastrophic Wastewater Failure in a Modern Metropolitan Area".
- https://www.rdpo.net/emergency-toilet
- Guidebook: https://docs.google.com/document/d/1Pt4W2ul_nfBeES0-aI38SFGN9_5-rCDcV45lbfvligA/edit

| Element | Specification | Source |
|---|---|---|
| Buckets | Two sturdy 5-6 gal buckets labeled PEE (#1) and POO (#2). Food-grade is not required; they must bear your weight. | RDPO |
| Seat | A toilet seat that fits or is designed for buckets. Two seats if possible. No pool noodles (can't be disinfected). | RDPO; Oregon OEM |
| Cover (layering) material | Carbon-based, dry, light and organic: sawdust, shredded paper, bark chips (not cedar), dry leaves or grass, peat moss, pet bedding. **About a handful per poo.** Wood pellets: 2 cups pellets + 1 cup water ≈ 6 cups sawdust. | RDPO |
| Bags | Heavy-duty garbage bags, **13-gallon, 0.9 mil or thicker**. Line the poo bucket. | RDPO |
| Fill rule | Fill **no more than half**, tie, and **double-bag**. No airtight lid, so contents can dry. | RDPO; Oregon OEM |
| Urine | Dilute with water if possible and pour on permeable ground or lawn, a different place each time. Toilet paper goes in the poo bucket. | RDPO; Oregon OEM |
| Storage and disposal | Store bags away from food, water, children, pets and pests. **Never** put them in curbside trash, yard-debris or recycling carts unless authorities say so. **Never bury bagged waste.** Wait for official collection instructions. | RDPO |
| Menstrual products and diapers | RDPO: menstrual products go in the poo bucket; diapers go in regular garbage. **Oregon OEM:** disposable pads and tampons go in a separate garbage bag, and cup contents go in the poo bucket or pit toilet. The two sources differ (§13). | RDPO; Oregon B2WR |
| Volume planning | Oregon OEM [paraphrase]: plan for **5 gallons of waste per person per week** when counting buckets. | https://www.oregon.gov/oem/Documents/B2WR-Complete-Tool-Kit-EN.pdf |
| Latrine alternative | Dig at least 2 ft (4 ft optimal). Cover each use with soil. Stop if you hit water. Only toilet paper goes in. | RDPO |

**DERIVED sizing (planning estimates):**
- **Bags.** Upper bound: 5 gal/person/week at half-fill of a 5-gal bucket (≈2.5 gal) = 2 poo bags/person/week, or 4 bag-units if double-bagged. Most urine goes to the pee bucket, so real use is lower. Suggested default: **1-2 bags/person/week, doubled for double-bagging**, i.e., **0.3-0.6 bags/person-day**. This is an assumption; label it as such.
- **Cover material.** About 1 handful (≈½-1 cup, assumed) per bowel movement, at about 1-1.5 movements/day (assumed), gives about **0.5-1.5 cups/person-day** (≈1-3 gal per person per 2 weeks). UNVERIFIED volumes.
- **Toilet paper.** Oregon OEM: measure your household's weekly use and double it for two weeks. There is no fixed per-person figure.

**Product costs** (Reliance Outdoors, 2026-09-25):
- Luggable Loo 5-gal toilet: **$25.99**.
- Double Doodie waste bags: $16.99 (bag count not captured, UNVERIFIED). Double Doodie Plus bags (hold up to 12 lb, with gel): $32.99.
- Bio-Gel: $18.99 (2 Tbsp gels 1 gal of liquid waste).
- https://relianceoutdoors.com/products.json

### 4.2 Hygiene items and hand-washing water

| Item | Quantity | Source |
|---|---|---|
| Bathing soap | **250 g per person per month** | Sphere 2018, Hygiene Promotion Std 1.2 key indicators [numbers] |
| Laundry soap | **200 g per person per month** | Sphere 2018 |
| Water containers | 2 per household, 10-20 L each (one to collect, one to store) | Sphere 2018 |
| Hand-washing station | Soap and water; 1 per household or per shared toilet | Sphere 2018 |
| Children's faeces | Potty, scoop or nappies | Sphere 2018 |
| Hand-washing water share | Public toilets: 1-2 L per user per day for hand washing, plus 2-8 L per cubicle per day for cleaning. Anal washing: 1-2 L per person per day where practiced. | Sphere WASH Appendix 3 |
| Hand washing | [PD-quote] "Wash your hands with soap and water for at least 20 seconds" ... "If you do not have soap and water, use an alcohol-based hand sanitizer that contains at least 60% alcohol." CDC (Aug 5, 2024) also suggests a temporary hand-washing station made from a large water jug. RDPO describes a siphon-pump hand-wash station. | https://www.cdc.gov/water-emergency/safety/guidelines-for-personal-hygiene-during-an-emergency.html |
| Ready.gov sanitation items | [PD-quote] "Moist towelettes, garbage bags and plastic ties (for personal sanitation)"; "Feminine supplies and personal hygiene items". | https://www.ready.gov/kit |

DERIVED: within the 1 gal/p/d baseline, about 0.25 gal (≈1 L) is the "sanitation" share. This is consistent with Sphere's 1-2 L/user for hand washing at toilets. Sphere's full 2-6 L hygiene share exceeds what the U.S. 1-gal guidance covers.

### 4.3 Bleach and disinfection

- **Surface disinfection.** CDC, *Cleaning and Disinfecting with Bleach* (Apr 24, 2024) [PD-quote]: "5 tablespoons (1/3 cup) of bleach per gallon of room temperature water or 4 teaspoons of bleach per quart of room temperature water".
  - Use bleach with 5-9% sodium hypochlorite, unscented, not "splashless".
  - Keep the surface visibly wet for **at least 1 minute**.
  - Make a fresh solution **daily**.
  - Never mix bleach with other cleaners. Ventilate.
  - https://www.cdc.gov/hygiene/about/cleaning-and-disinfecting-with-bleach.html
- Water disinfection doses are in §1.4. Container sanitizing is 1 tsp per quart (§1.3).
- DERIVED bleach stock: surface solution at ⅓ cup per gallon per day of cleaning, plus water disinfection at 8 drops per gallon. One 64-121 oz bottle covers typical 2-week household needs (assumption). Note that bleach loses strength over time; a specific shelf life is UNVERIFIED here.

### 4.4 Menstrual supplies

- **CDC factsheet** *Managing Your Period During a Natural Disaster* (updated Sept 11, 2026) [PD]: keep **"Period products for 2 cycles"** in the emergency kit, along with soap or hand sanitizer, clean underwear and pain relievers. [PD-quote] "Change tampons every 4 to 8 hours. Do not wear a single tampon for more than 8 hours at a time." The factsheet also covers making a substitute pad from clean cloth.
  - https://www.cdc.gov/natural-disasters/media/pdfs/2026/09/FS-Managing-Period-During-Disaster-508.pdf
  - https://www.cdc.gov/hygiene/about/menstrual-hygiene.html
- **Sphere 2018**, Hygiene Promotion Std 1.3 [numbers], per menstruating person, whichever product they prefer:
  - absorbent cotton material **4 m² per year**, OR
  - **15 disposable pads per month**, OR
  - **6 reusable pads per year**.
  - Plus **6 pairs of underwear per year** and an extra **250 g of soap per month**.
- DERIVED: at 3-6 products/day (the 4-8 h tampon rule) × ~5 bleeding days (an assumed typical duration), that is 15-30 products per cycle. The **CDC "2 cycles"** rule is 30-60 products per menstruating person for kits up to 1 month. For longer tiers, add 1 cycle per 28 days (assumption).

### 4.5 Diapers and wipes

| Age | Diapers per day | Per 30 days (DERIVED) |
|---|---|---|
| 0-1 mo | 8-12 | 240-360 |
| 2-4 mo | 8-10 | 240-300 |
| 5-8 mo | 7-9 | 210-270 |
| 9-12 mo | 6-8 | 180-240 |
| Toddlers over 12 months | UNVERIFIED (not captured) | — |

Source: Pampers guide (manufacturer) [numbers], https://www.pampers.com/en-us/baby/diapering/article/how-many-diapers-a-day. Sutter Health: after the first week, expect more than six wet diapers a day (a hydration check), https://www.sutterhealth.org/health/how-many-diapers-a-day-your-newborn-should-have.

- **CDC checklist** [PD]: "At least one large pack of diapers", "At least two packs of baby wipes", diaper rash cream, gallon resealable bags for dirty diapers and clothes, a wash basin and scrub brush for feeding items.
- RDPO: diapers go in regular garbage, not the twin bucket.

---

## 5. SHELTER, HEAT, COOLING

### 5.1 Carbon monoxide (non-negotiable rules)

CDC, *Carbon Monoxide Poisoning Basics* (Jan 12, 2026) [PD-quote]:
- "Never use a generator inside your home or garage, even if doors and windows are open."
- "Only use generators outside, more than 20 feet away from any windows, doors, and vents."
- "Never heat your house with a gas oven."
- "Never burn charcoal indoors."
- "Never use a portable gas camp stove indoors."
- "Do not use portable flameless chemical heaters indoors."
- Install battery-operated or battery-backup CO detectors near every sleeping area. Check batteries at each clock change. Replace detectors per the manufacturer or every 5 years.
- Symptoms: headache, dizziness, weakness, upset stomach, vomiting, chest pain, confusion. People asleep or intoxicated can die before they have symptoms.
- Toll: more than 400 non-fire CO deaths per year in the U.S. A CDC PSA page says more than 500 (§13).

https://www.cdc.gov/carbon-monoxide/about/index.html ; https://www.cdc.gov/natural-disasters/psa-toolkit/avoiding-carbon-monoxide-poisoning.html

Ready.gov (Power Outages) [PD-quote]: "Generators and fuel should always be used outdoors and at least 20 feet away from windows, doors and attached garages." / "Install working carbon monoxide detectors on every level of your home."

**Engine implication (DERIVED):** apartments and condos without a safe outdoor spot 20 ft from openings should not be offered a fuel generator. Route those users to battery power stations.

### 5.2 Safe heating and warming a single room

CDC, *Safety Guidelines: During & After a Winter Storm* (Aug 25, 2026) [PD]:

- **Backup heat.** Have at least one of the following:
  - extra blankets, sleeping bags and warm winter coats;
  - a fireplace up to code with plenty of dry firewood, or a gas-log fireplace;
  - portable space heaters or kerosene heaters (check with your local fire department whether kerosene heaters are legal).
- **Space heaters.** [PD-quote] "Use electric space heaters with automatic shut-off switches and non-glowing elements."
  - Keep them 3 ft from anything flammable.
  - Never place them on furniture or near water.
  - Avoid extension cords.
- **Combustion heaters.** Use only if properly vented to the outside. Give kerosene heaters proper ventilation. Use only the fuel the heater is designed for.
- **Conserve heat (the single-room strategy)** [PD-quote]:
  - "Close off unneeded rooms."
  - "Stuff towels or rags in cracks under doors."
  - "Close draperies or cover windows with blankets at night."
  - Avoid opening doors unnecessarily.
- **Babies.** [PD-quote] "Infants less than one year old should never sleep in a cold room". Use warm sleepwear or sleep sacks, remove soft bedding, and move elsewhere if the home can't be kept warm.
- **Older adults.** Check on them often. People over 65 should check their home temperature often.
- **Hypothermia.** A temperature below 95°F is an emergency. Warm the core (chest, neck, head, groin).
- **Stranded in a car.** Run the engine about 10 minutes per hour with a window slightly open and the exhaust pipe clear of snow.

https://www.cdc.gov/winter-weather/safety/stay-safe-during-after-a-winter-storm-safety.html

Other sources:
- **Ready.gov kit** [PD-quote]: "Sleeping bag or warm blanket for each person".
- **Lehi City (UT) Fire** [paraphrase]: the International Fire Code prohibits unvented portable kerosene heaters in occupied living spaces, and portable units are limited to a 2-gallon tank. This conflicts in tone with CDC listing kerosene heaters as an option (§13). https://www.lehi-ut.gov/departments/fire-department/fire-prevention/home-fuel-storage-limits/
- **Sphere 2018** [numbers]: at least 1 blanket and bedding per person, with extra blankets and ground insulation in cold climates. Covered living space is 3.5 m² per person, or 4.5-5.5 m² in cold or urban settings. These are humanitarian shelter figures, included for reference.

### 5.3 Cooling and extreme heat

- **CDC, *About Heat and Your Health*** (July 20, 2026) [PD-quote]: "Use fans, but only if indoor temperatures are less than 90°F. In temperatures above 90°F, a fan can increase body temperature."
  - Use air conditioning, or find a cooling center (dial 2-1-1).
  - Carry water.
  - Light yellow or clear urine usually means adequate hydration.
  - High-risk groups: children with asthma, heart disease, pregnancy, age 65+, outdoor workers, infants and young children.
  - Plan for refrigerated medications and powered medical devices.
  - https://www.cdc.gov/heat-health/about/index.html
- **Ready.gov, Extreme Heat** (07/16/2026) [PD-quote]: "Fans create air flow and a false sense of comfort, but do not reduce body temperature or prevent heat-related illnesses."
  - Find a cooling center; take cool showers.
  - [PD-quote] "Cover windows with drapes or shades"; "Use window reflectors specifically designed to reflect heat back outside"; "Weather-strip doors and windows".
  - Heat stroke signs include body temperature above 103°F; call 9-1-1.
  - Never leave people or pets in a closed car.
  - https://www.ready.gov/heat
- NIOSH hydration limits are in §1.2.

### 5.4 Shelter-in-place: plastic sheeting and duct tape

Ready.gov, *Shelter* (09/24/2026) [PD-quote]:
- "Go into an interior room with few windows if possible."
- "Turn off fans, air conditioning and forced air heating systems."
- "Seal all windows, doors and air vents with thick plastic sheeting and duct tape."
- "Cut the plastic sheeting several inches wider than the openings and label each sheet."
- "Consider measuring and cutting the sheeting in advance to save time."
- "Duct tape plastic at corners first and then tape down all edges."

The kit list says [PD-quote] "Plastic sheeting, scissors and duct tape (to shelter in place)". https://www.ready.gov/shelter

- Ready.gov gives no plastic thickness (mil) and no air-per-person figure (UNVERIFIED).
- DERIVED sizing:
  - sheeting area = Σ over windows, doors and vents of (width + 0.5 ft) × (height + 0.5 ft), plus 20% waste;
  - tape length ≈ 2 × Σ perimeters of the cut sheets, plus 20%.
  - The user enters the room's openings.
- Tarps: not on the Ready.gov basic list. ASPCA's human kit suggests tarp, rope and duct tape [paraphrase]. Oregon B2WR's shelter plan assumes sheltering up to 2 weeks, including outdoors (tents, sleeping bags).

---

## 6. POWER

### 6.1 Daily energy by load

| Load | Daily energy (Wh/day) | Source and basis |
|---|---|---|
| Refrigerator, top-freezer (ENERGY STAR certified; median of 832 models) | **≈995** (363 kWh/yr; p10-p90 297-450) | EPA ENERGY STAR Certified Residential Refrigerators dataset p5st-her9, queried 2026-09-25: https://data.energystar.gov/resource/p5st-her9.json |
| Refrigerator, bottom-freezer (n=1,660) | ≈1,523 (556 kWh/yr) | same |
| Refrigerator, side-by-side (n=123) | ≈1,753 (640 kWh/yr) | same |
| Compact refrigerator (n=1,741) | ≈674 (246 kWh/yr) | same |
| Upright freezer, auto-defrost, class 9 (n=331, median 17 ft³) | ≈1,192 (435 kWh/yr) | ENERGY STAR Residential Freezers dataset 8t9c-g3tn: https://data.energystar.gov/resource/8t9c-g3tn.json |
| Chest freezer, class 10 (n=9, median 10.6 ft³) / compact chest (n=40, ~5 ft³) | ≈597 / ≈537 (218 / 196 kWh/yr) | same |
| Rule of thumb for any appliance | [PD-quote] "(Wattage × Hours Used Per Day) ÷ 1000 = Daily Kilowatt-hour (kWh) consumption". For refrigerators, DOE says to divide plugged-in time by three to estimate hours at maximum wattage. | DOE Energy Saver (Wayback 2024-12-31): https://web.archive.org/web/20241231180235/https://www.energy.gov/energysaver/estimating-appliance-and-home-electronic-energy-use |
| CPAP (ResMed AirSense 11), humidifier on | ≈**170 Wh/night** (≈14 Ah at 12 V over 7 h) | User-measured, SIL "CPAP Power Options" [paraphrase]: https://power.sil.org/cpap-power-options/ . The AirSense 11 uses a 65 W, 24 V supply (CPAP Shop). Manufacturer spec not fetched (UNVERIFIED). |
| CPAP, humidifier off | ≈**<100 Wh/night** (<8 Ah at 12 V) | SIL. SIL relays that battery makers recommend not using the humidifier on battery. |
| Travel CPAP (ResMed AirMini) | 33-99 Wh/night | SIL |
| Laptop, full charge | MacBook Air M5: **53.8 Wh** (13") / **66.5 Wh** (15") | https://www.apple.com/macbook-air/specs/ |
| Smartphone, full charge | ≈10-20 Wh (**UNVERIFIED**: derived from typical 2,500-5,000 mAh at ~3.85 V; no manufacturer Wh spec captured) | — |
| Well pump, ½ hp submersible | ≈1,000-1,500 W running; 2-3× that to start (**UNVERIFIED**: secondary web sources, including one quoting a Generac manual at 1,500 W running for ½ hp). Daily Wh = running W × pump hours. | Check the pump nameplate |
| Oxygen concentrator and other powered medical devices | **UNVERIFIED** (not fetched). Use the device label. Ready.gov: talk to your provider, keep extra batteries charged, and ask your utility about a priority-restoration list. | https://www.ready.gov/disability ; https://www.ready.gov/power-outages |

**Cold-retention alternative to powering a refrigerator.** Ready.gov: a closed fridge holds about 4 hours and a full freezer about 48 hours. The engine can recommend "keep closed + dry ice" (25 lb holds a 10 ft³ freezer 3-4 days) instead of generator hours.

### 6.2 What a small power station and a generator deliver

**Power stations:**

| Unit | Capacity | Output | Other specs | Price |
|---|---|---|---|---|
| Jackery Explorer 1000 v2 | **1,070 Wh** LiFePO4 | **1,500 W** rated, 3,000 W surge | 4,000 cycles to at least 70% capacity | **$559** observed on product page 2026-09-25 (volatile) |
| Anker SOLIX C1000 | 1,056 Wh | 1,800 W | LFP, 3,000 cycles | not pinned (variant prices ambiguous) |
| Westinghouse iGen400s | small | — | — | $169 |

Sources: https://www.jackery.com/products/jackery-explorer-1000-v2 ; https://www.anker.com/products/a1761 ; https://westinghouseoutdoorpower.com/products.json

DERIVED runtimes on 1,070 Wh, assuming 85% usable after inverter losses (assumption):

| Load | Runtime |
|---|---|
| Top-freezer fridge (995 Wh/day) | ≈0.9 day |
| Chest freezer (597 Wh/day) | ≈1.5 days |
| CPAP with humidifier (170 Wh/night) | ≈5.4 nights |
| CPAP without humidifier (96 Wh/night) | ≈9.5 nights |
| Phones only (4 × 15 Wh/day) | ≈15 days |

**Honda EU2200i inverter generator** (Honda page, Wayback 2026-09-14): **$1,199** MSRP. 2,200 W max, 1,800 W rated. Fuel tank **0.95 gal**. Run time **3.2 h at rated load** and **8.1 h at ¼ load**. Noise 48-57 dB(A). https://web.archive.org/web/20260914102101/https://powerequipment.honda.com/generators/models/eu2200i

DERIVED from the Honda figures:

| Operating point | Fuel per hour | Fuel per 24 h continuous | Delivered energy per gallon |
|---|---|---|---|
| ¼ load (450 W) | 0.117 gal/h | **2.8 gal/day** | ≈3.8 kWh/gal |
| Rated (1,800 W) | 0.297 gal/h | **7.1 gal/day** | ≈6.1 kWh/gal |

Gasoline holds 120,214 Btu/gal (≈35.2 kWh thermal, per EIA), so light-load electrical efficiency is only about 11%.

Budget generators (Westinghouse, 2026-09-25): iGen2550 inverter **$499**; iGen2800c inverter with CO sensor $549; WGen3600DFv dual-fuel (3,600 running W per listing) $499.

**Generator safety** (Ready.gov, CDC):
- Outdoors only, at least 20 ft from openings.
- Keep it dry.
- Let it cool before refueling.
- Plug appliances in with heavy-duty outdoor-rated cords.
- Do not store gasoline indoors (CDC winter page).

### 6.3 Fuel storage safety and shelf life

**Residential limits.** Lehi City, Utah, Fire Prevention, *Home Fuel Storage Limits* [paraphrase; based on IFC/NFPA; local codes vary]:
- **Flammable liquids (gasoline, white gas):**
  - at most **25 gal** total, preferably in a detached garage or shed;
  - at most **10 gal** in an attached garage;
  - **none** in the living space;
  - approved containers only, and empty cans count as full;
  - above 5 gal, keep a 2A10BC extinguisher 10-50 ft away.
  - Lehi recommends keeping it to 5 gal.
- **Propane (portable DOT cylinders):**
  - up to **25 gal** total capacity, e.g., five 20-lb cylinders or one 100-lb cylinder, stored outside or in an unattached shed;
  - at most **two 1-lb disposable cylinders** inside the home or attached garage (NFPA 58).
- **Diesel, kerosene, lamp oil:** 60 gal outside the residence. The page's attached-garage figure is internally inconsistent (25 vs 10 gal).

**Shelf life.**
- Lehi [paraphrase]: fuels do not have an indefinite shelf life, many appliance makers recommend using fuel within **6 months** of purchase, and stored fuel must be rotated.
- Gasoline untreated for 3-6 months, or 1-2 years with stabilizer: secondary web sources only (**UNVERIFIED**). Honda's owner-manual wording was not fetched.
- Propane does not chemically degrade: no authoritative page captured (**UNVERIFIED**). **Cylinder requalification** applies: DOT 4B/4BA/4BW cylinders in non-corrosive service have a **12-year** initial requalification interval under 49 CFR 180.209 [PD; summary]. In practice, refillers may refuse cylinders past their date stamp. https://www.ecfr.gov/current/title-49/section-180.209

### 6.4 Solar realism

NREL, now **NLR**, PVWatts v8 (v8.5.0) API output for a **100 W** fixed array: 30° tilt, facing south, default 14% system losses, AC output. Queried 2026-09-25 via https://developer.nlr.gov/api/pvwatts/v8.json (the NREL domains no longer resolve).

| City | Annual kWh per 100 W | Avg Wh/day | **December Wh/day** | Best month Wh/day | Panel W to run one top-freezer fridge (995 Wh/day): avg / Dec (DERIVED) |
|---|---|---|---|---|---|
| Seattle, WA | 111.9 | 307 | **121** | 466 | 324 W / **822 W** |
| Philadelphia, PA | 139.2 | 381 | **261** | 452 | 261 W / 381 W |
| Minneapolis, MN | 138.1 | 378 | **219** | 492 | 263 W / 454 W |
| Miami, FL | 154.8 | 424 | **382** | 495 | 235 W / 260 W |
| Phoenix, AZ | 179.8 | 493 | **424** | 567 | 202 W / 235 W |

**Reality check.** A portable 100-200 W panel keeps phones, lights, radios and a CPAP going almost everywhere. It does **not** reliably run a full-size refrigerator in northern winters without about 400-800 W of panels plus storage. PVWatts assumes ideal orientation and no shading. Portable panels laid flat or partly shaded will produce less (magnitude UNVERIFIED).

---

## 7. COMMUNICATIONS AND INFORMATION

| Channel | Key facts | Source |
|---|---|---|
| **NOAA Weather Radio All Hazards** | [PD-quote] "NWR includes more than 1000 transmitters, covering all 50 states, adjacent coastal waters, Puerto Rico, the U.S. Virgin Islands, and the U.S. Pacific Territories." Seven VHF frequencies, 162.400-162.550 MHz. Broadcasts 24/7. Requires a special receiver. Ready.gov lists a "Battery-powered or hand crank radio and a NOAA Weather Radio with tone alert". | https://www.weather.gov/nwr/ ; https://www.ready.gov/kit |
| NWR receiver prices (Midland, 2026-09-25) | WR120 **$54.99**; WR400 $109.99; ER310 crank radio with power bank $89.99; ER10 compact $34.99 | https://midlandusa.com/products.json |
| **Wireless Emergency Alerts (WEA)** | 47 CFR 10.430 [PD]: carriers must support alerts up to **360 characters**, or 90 on legacy network elements. Ready.gov [PD-quote]: "You are not charged for receiving WEAs, and there is no need to subscribe". Senders are state, local, tribal and territorial officials, NWS, NCMEC and the President. FCC [PD]: carrier participation is voluntary. To receive alerts the phone must be WEA-capable, on, not in airplane mode, and served by a participating carrier. | https://www.ecfr.gov/current/title-47/section-10.430 ; https://www.ready.gov/alerts ; FCC WEA (Wayback 2026) https://web.archive.org/web/2026/https://www.fcc.gov/consumers/guides/wireless-emergency-alerts-wea |
| **Emergency Alert System (EAS)** | Ready.gov [PD-quote]: allows "the president to address the nation within 10 minutes". | https://www.ready.gov/alerts |
| **FRS** (Family Radio Service) | FCC [PD]: licensed by rule, so **no license** and no age limit. 22 channels, all shared with GMRS. **2 W ERP** on channels 1-7 and 15-22; **0.5 W** on channels 8-14. Channels 8-14 usually reach **less than half a mile**; the others reach farther. No phone interconnect. Two-pack example: Midland T71 FRS $99.99. | FCC FRS (Wayback 2026) https://web.archive.org/web/2026/https://www.fcc.gov/wireless/bureau-divisions/mobility-division/family-radio-service-frs |
| **GMRS** | FCC [PD]: **license required**, applicant must be 18+, and any family member may then operate. **10-year term.** 30 channels. [PD-quote] "A GMRS user can expect a communications range of one to twenty-five miles depending on station class, terrain, and repeater use." Repeaters may not be linked over the internet. Short texts and GPS data have been allowed since 2017. **Fee: $35** for a new license or renewal (47 CFR 1.1102, as of 2026-09-01). Radio examples: Midland GXT1000 $49.99 each; GXT3000 2-pack $169.99. | FCC GMRS (Wayback 2026) https://web.archive.org/web/2026/https://www.fcc.gov/wireless/bureau-divisions/mobility-division/general-mobile-radio-service-gmrs ; https://www.ecfr.gov/current/title-47/section-1.1102 |
| **Amateur (ham) radio** | FCC [PD]: license by exam through Volunteer Examiners; most people start at Technician class. The FCC personal-license application fee is **$35** (47 CFR 1.1102). The VE session fee is **UNVERIFIED**. | FCC Amateur (Wayback 2026) https://web.archive.org/web/2026/https://www.fcc.gov/wireless/bureau-divisions/mobility-division/amateur-radio-service |
| **Meshtastic** (LoRa mesh) | [paraphrase] Open-source off-grid **text messaging** and optional GPS location over inexpensive LoRa radios. Typically several kilometers per hop; the 331 km record is not typical. No license needed on unlicensed ISM bands (U.S. 902-928 MHz). Each radio pairs with one phone at a time. Messages flood through the mesh with a hop limit (3-bit field, max 7). Default "LongFast" preset is about **1.07 kbps** theoretical, with at most 237 bytes of payload per packet. Ham-licensed mode prohibits encryption and requires call-sign ID. Realistic use: neighborhood and family text check-ins when cell networks are down, **if** enough nodes are deployed with line of sight in advance. It is not a substitute for 911, WEA or NWR. Node prices **UNVERIFIED**. | https://meshtastic.org/docs/introduction/ ; https://meshtastic.org/docs/faq/ ; https://meshtastic.org/docs/overview/radio-settings/ ; https://meshtastic.org/docs/overview/mesh-algo/ |
| **Family communication plan** | Ready.gov Plan (09/01/2026) [PD]: know how you will contact each other, set a family meeting place, and account for ages, medical needs, disabilities, pets, diet and school-age children. There is a fillable card PDF. Ready.gov Low-cost page [PD-quote]: "Make sure you store important phone numbers somewhere besides just your cell phone." Ready.gov Earthquakes [PD]: choose an out-of-state contact; texting may be more reliable than calls and saves battery. Ready.gov Disability [PD]: keep the contact list in a watertight container in the kit. | https://www.ready.gov/plan ; https://www.ready.gov/sites/default/files/2025-06/family-communication-plan_fillable-card.pdf ; https://www.ready.gov/low-and-no-cost ; https://www.ready.gov/earthquakes |
| **Maps and offline information** | Ready.gov kit [PD]: "Local maps". Red Cross [paraphrase]: map(s) of the area. No quantitative guidance; offline phone maps are not addressed by authorities. | — |

---

## 8. DOCUMENTS AND FINANCE

### 8.1 Emergency Financial First Aid Kit (EFFAK)

FEMA P-1075, September 2019 (3rd edition, April 2019). The content is copyright Operation HOPE, so it is paraphrased here. PDF retrieved via Ready.gov: https://www.ready.gov/sites/default/files/2020-03/ready_emergency-financial-first-aid-toolkit.pdf

**Four categories of checklists and forms:**
1. **Household Identification**
2. **Financial and Legal Documentation**
3. **Medical Information**
4. **Household Contacts**

(Ready.gov's page lists "Insurance Information" separately. In the PDF, insurance sits inside the financial and legal category.)

**Four steps:**
1. **Assess and compile.** This step includes photos or video of your home and belongings, and keeping cash in the same safe place as the EFFAK.
2. **Review** insurance and paperwork.
3. **Safeguard** paper and electronic copies: a fireproof and waterproof box or safe, a safe deposit box, or a trusted person; password-protected files on a removable drive; or secure offsite storage.
4. **Update** the kit at tax time, at daylight-saving changes, on your birthday, and at the new year.

**Cash on hand.** EFFAK: base the amount on your family's basic needs for food, gas and daily items. Ready.gov (Financial Preparedness, 03/13/2026) [PD-quote]: "Keep a small amount of cash at home in a safe place. It is important to have small bills on hand because ATMs and credit cards may not work during a disaster when you need to purchase necessary supplies, fuel or food." **No authority gives a dollar figure.** DERIVED: `cash = tier_days (capped, e.g., 3-14) × household daily cash spend (food + fuel + incidentals)`, entered by the user. https://www.ready.gov/financial-preparedness

**Benefits.** Ready.gov Older Adults [PD]: switch federal benefits to direct deposit or the Direct Express card, because disasters can disrupt mail for days or weeks.

### 8.2 Insurance checks

| Check | Evidence | Source |
|---|---|---|
| Flood (NFIP) waiting period | [PD-quote] "there is typically a 30-day waiting period for an NFIP policy to go into effect, unless the coverage is mandated ... or is related to a community flood map change." | FEMA Flood Insurance (Wayback 2026) https://web.archive.org/web/2026/https://www.fema.gov/flood-insurance |
| Flood not covered by standard policies | Ready.gov [PD-quote]: "Homeowners insurance does not typically cover flooding". EFFAK [paraphrase]: flood damage is rarely covered by homeowners or renters policies, and many policies only take effect 30 days after signing. | https://www.ready.gov/financial-preparedness |
| NFIP limits | FloodSmart [PD]: residential building coverage **up to $250,000**; contents **up to $100,000**. Renters can buy contents-only coverage. | https://www.floodsmart.gov/get-insured/buy-a-policy |
| Earthquake | Ready.gov Earthquakes [PD-quote]: "A standard homeowner's insurance policy does not cover earthquake damage." | https://www.ready.gov/earthquakes |
| Renters | EFFAK [paraphrase]: verify your renters insurance is current; see usa.gov/property-insurance. | EFFAK |
| Umbrella liability | No authoritative preparedness source located (**UNVERIFIED** as a preparedness item). | — |

### 8.3 Emergency fund

| Source | Position | Link |
|---|---|---|
| **CFPB**, *An essential guide to building an emergency fund* | [PD] **No fixed number.** [PD-quote] "The amount you need to have in an emergency savings fund depends on your situation." It encourages starting small, automatic saving, and saving part of a tax refund. | https://www.consumerfinance.gov/an-essential-guide-to-building-an-emergency-fund/ |
| **FINRA**, *Financial Foundations* | [paraphrase] Financial planners often recommend **3-6 months of living expenses**. People with variable income may need more. Keep it liquid. | https://www.finra.org/investors/investing/investing-basics/financial-foundations |
| **Federal Reserve Bank of St. Louis**, *Page One Economics* (Sept 2025) | [paraphrase] Experts often recommend **3-6 months of essential expenses**. It cites the Fed's 2024 SHED survey: 63% of adults would cover a $400 emergency with cash or its equivalent, and **55%** had 3 months of expenses set aside. | https://www.stlouisfed.org/publications/page-one-economics/2025/sep/when-unexpected-happens-be-ready-with-emergency-fund |

**Why it belongs in a preparedness app.** EFFAK [paraphrase] names lack of income and savings as the main obstacle to building a rainy-day fund, stockpiling supplies, or buying insurance. Disasters bring deductibles, evacuation lodging, lost wages and waiting periods. The 2007 Church pamphlet also treats a financial reserve as one of four home-storage pillars.

---

## 9. EVACUATION

### 9.1 Go-bag and fuel rules

| Source | Guidance |
|---|---|
| **Ready.gov, Evacuation** (03/20/2026) | [PD-quote] "Keep a full tank of gas if an evacuation seems likely. Keep a half tank of gas in it at all times in case of an unexpected need to evacuate." Also: "Build a go-bag"; "Make sure you have a portable emergency kit in the car."; "Take your pets with you but understand that only service animals may be allowed in public shelters."; "Leave a note telling others when you left and where you are going."; follow official routes. https://www.ready.gov/evacuation |
| **CDC evacuation PSA** | [PD-quote] "Only take what you really need with you: cell phone, chargers, medicines, ID, cash, and car emergency kit." Also "Know where you can evacuate with your pet." https://www.cdc.gov/natural-disasters/psa-toolkit/be-prepared-in-case-you-need-to-evacuate.html |
| **Red Cross** | [paraphrase] 3-day supply of water and food in an easy-to-carry kit for evacuation. |
| **Washington EMD** | [paraphrase] Evacuation kit for 2-3 days, lightweight. One kit per person, with children making their own. Food that needs no refrigeration or cooking. Pets get a 2-3 day kit. |
| **Ready.gov kit contents** | The Basic Disaster Supplies Kit list in §3.5 and §7, plus [PD-quote] "Complete change of clothing appropriate for your climate and sturdy shoes", "Cash or traveler's checks", "Important family documents", "Whistle (to signal for help)", "Flashlight", "Extra batteries". https://www.ready.gov/kit |

### 9.2 Vehicle kit

- **Ready.gov Winter Weather** (03/10/2026) [PD-quote]: "Create an emergency supply kit for your car. Include jumper cables, sand, a flashlight, warm clothes, blankets, bottled water and non-perishable snacks. Keep a full tank of gas." https://www.ready.gov/winter-weather
- **CDC winter** [PD]:
  - Check and restock winter car supplies before trips.
  - Carry extra warm clothing and blankets, because the car may break down.
  - If stranded: tie a bright cloth to the antenna, run the engine about 10 minutes per hour with a window cracked, and keep the exhaust pipe clear.
- **Price examples:** Red Cross Deluxe 137-piece Auto First Aid Kit $31.75; My Patriot Supply "Survival Car Kit" $129.95.

### 9.3 Get-home bag (commuter)

- **Historical Ready.gov guidance** (Build a Kit, 2020 capture) [PD-quote]: "Work: Be prepared to shelter at work for at least 24 hours. Your work kit should include food, water and other necessities like medicines, as well as comfortable walking shoes, stored in a 'grab and go' case." https://web.archive.org/web/20200101090440/https://www.ready.gov/kit
- **Walking speed.** MUTCD 2009 §4E.06 [PD] designs pedestrian clearance for **3.5 ft/s (≈2.4 mph)**, and **3 ft/s (≈2.0 mph)** for total crossing time. Use 2.0-2.4 mph as a conservative loaded or night pace. https://mutcd.fhwa.dot.gov/htm/2009/part4/part4e.htm
- **Water while walking.** NIOSH (moderate activity in heat, under 2 h): 8 oz every 15-20 min, i.e., **24-32 oz/hour**, and not more than 6 cups/hour. In temperate weather, drink to thirst; the CDC Yellow Book warns against forced overhydration.

DERIVED water for the walk home (hot conditions, 24-32 oz/h):

| Distance | Hours at 2.4 / 2.0 mph | Water at 2.4 mph | Water at 2.0 mph |
|---|---|---|---|
| 5 mi | 2.1 / 2.5 h | 1.5-2.0 L | 1.8-2.4 L |
| 10 mi | 4.2 / 5.0 h | 3.0-3.9 L | 3.6-4.7 L |
| 15 mi | 6.2 / 7.5 h | 4.4-5.9 L | 5.3-7.1 L |
| 20 mi | 8.3 / 10.0 h | 5.9-7.9 L | 7.1-9.5 L |
| 25 mi | 10.4 / 12.5 h | 7.4-9.9 L | 8.9-11.8 L |

Recommendation: past about 2-3 L, carry a filter or tablets (§1.4) plus containers instead of all the water.

**Other get-home items with sources:**
- sturdy or comfortable walking shoes (Ready.gov);
- flashlight and extra batteries (Ready.gov);
- local map (Ready.gov);
- cash in small bills (Ready.gov);
- whistle and dust mask (Ready.gov);
- personal medications (Ready.gov work kit);
- phone charger or backup battery (Ready.gov: "Cell phone with chargers and a backup battery").

Food: DERIVED from the DGA moderately active adult (2,000-2,800 kcal) → about 1,000-1,500 kcal per 12-hour walk day (assumption).

### 9.4 Pet evacuation

- **ASPCA** [paraphrase]:
  - Put a rescue-alert sticker on the home; write "EVACUATED" on it if time allows.
  - Arrange pet-friendly lodging ahead of time.
  - The Evac-Pack holds **7-10 days of food** (rotate every 2 months), **at least 7 days of bottled water per person and pet** (replace every 2 months), a **2-week supply of medicine**, medical records, a carrier for each pet, and recent photos.
  - Plan for the worst case: even for a one-day evacuation, assume weeks away.
  - https://www.aspca.org/pet-care/general-pet-care/disaster-preparedness
- **Ready.gov Pets** (03/20/2026) [PD-quote]: "Keep several days' supply of food in an airtight, waterproof container"; "Store a water bowl and several days' supply of water"; "Keep an extra supply of the medicine your pet takes on a regular basis in a waterproof container"; "Pet litter and litter box (if appropriate), newspapers, paper towels, plastic trash bags and household chlorine bleach". Also [PD-quote] "If local officials ask you to evacuate, that means your pet should evacuate too." https://www.ready.gov/pets

### 9.5 Documents to take

The EFFAK checklists (§8.1). The CDC pregnancy page [PD] also lists medical records, insurance information, ID cards, all prescription information, and medical supplies, kept in a waterproof, portable container.

---

## 10. SPECIAL NEEDS

| Group | Key quantities and guidance | Source |
|---|---|---|
| **Infants** | Breastfeeding first. Ready-to-feed formula is the safest formula. 2½ oz formula per lb per day, max 32 oz. Diapers 8-12/day in the first month. Kit per the CDC checklist, including 1-2 boxes of nursing pads, a manual pump, at least 1 large pack of diapers, at least 2 packs of wipes, infant acetaminophen, an infant thermometer, and a portable crib. Infants must not sleep in cold rooms. | §2.5, §4.5, §5.2 |
| **Older adults** | Ready.gov [PD]: assess needs; keep NOAA radio tuned; plan transport if you need help evacuating; include medicines, supplies, batteries and chargers; copy Medicare and Medicaid cards; give someone in your network a key and training on equipment; find out a dialysis or treatment clinic's backup provider; get benefits electronically. There is also a FEMA guide, "Take Control in 1, 2, 3". Calorie needs fall with age (DGA). Heat and cold risk are higher (CDC). | https://www.ready.gov/older-adults |
| **Disability / access and functional needs** | Ready.gov (09/09/2026) [PD]: create an emergency medication supply with your doctor or pharmacist; keep a medication list; have a plan for power-dependent equipment and ask your utility about priority restoration; buy an extra battery for a power wheelchair; keep "A backup supply of oxygen"; know more than one dialysis facility; pack service-animal supplies; carry printed communication cards; build a support network with a contact list in a watertight container. | https://www.ready.gov/disability |
| **Pregnancy / postpartum** | CDC [PD]: at least 3 days of food and water, and try for 2 weeks of water; **7-10 days of prescriptions** and ask for a **30-day** emergency refill; menstrual products; several months of contraception (pill, patch, ring) or a long-acting method; learn the urgent maternal warning signs; avoid iodine-treated water. DGA: +340 / +452 kcal in the 2nd / 3rd trimester. DRI water 3.0 L (pregnancy) and 3.8 L (lactation). Heat raises risk (CDC). | §3.2; https://www.cdc.gov/reproductive-health/emergency-preparation-response/safety-messages.html |
| **Diabetes / insulin** | At least 1-2 weeks of supplies. Insulin cool, never frozen. Discard afterwards if stored at extreme temperatures. Glucose tablets or 15 g quick carbs. | §3.2-3.3 |
| **Dialysis** | CDC links a 3-day emergency diet for people who miss dialysis. Ready.gov: know alternate facilities. | https://www.cdc.gov/diabetes/articles/diabetes-care-emergencies.html |
| **Pets** | Food: Ready.gov "several days"; ASPCA 7-10 days. Water: dogs about 1 oz/lb/day; cats about 4 oz per 5 lb/day. Medicine: ASPCA 2 weeks. Litter, carrier, ID/microchip, photos, records. | §1.2, §9.4 |
| **Livestock / horses** | Water: 20-30 L per large or medium animal per day; 5 L per small animal (Sphere). ASPCA [paraphrase]: evacuate early, set up a phone tree or buddy system with nearby farms and share trailers, keep vet records handy, fill tubs ahead of power loss. | Sphere App. 3; ASPCA |

---

## 11. TIER DEFINITIONS AND EACH AUTHORITY'S RATIONALE

| Tier | Authority | What they say | Stated rationale | Source |
|---|---|---|---|---|
| **72 hours** | FEMA / Ready.gov (historical, through at least July 2021) | [PD-quote] "supplies to last for at least 72 hours"; water "for at least three days"; food "at least a three-day supply". In 2022 or later this became "several days". | [PD-quote, 2020 page] "After an emergency, you may need to survive on your own for several days." | Wayback links in §1.1 |
| 72 hours | CDC water page | 3 days minimum, "try" 2 weeks | Tap water may be unavailable or unsafe | §1.1 |
| 72 hours | EU Preparedness Union Strategy, JOIN(2025) 130 (Mar 2025) | [paraphrase] Key action: guidelines for population self-sufficiency of **at least 72 hours** | The initial period of an extreme disruption is the most critical | https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX:52025JC0130 |
| 72 hours | Denmark BRS | [paraphrase] Be able to manage **3 days** | Frees authorities to stabilize the situation and help those most in need | https://www.brs.dk/en/prepared/ |
| 3 days evacuation + **2 weeks home** | American Red Cross | [paraphrase] Water and food: 3-day supply for evacuation, 2-week supply for home. Medications: 7 days. | (not stated on page) | Red Cross Wayback link |
| **1 week** | Sweden (MSB "Sju dagar", Hemberedskap) | [paraphrase] Practice handling a crisis for **one week** | — | https://www.msb.se/sv/amnesomraden/msbs-arbete-vid-olyckor-kriser-och-krig/om-krisen-eller-kriget-kommer/ |
| **10 days** | Germany BBK (11/2025) | [paraphrase] All households should if possible be self-sufficient for **10 days**; at least 3 days already helps a lot | — | §1.1 BBK PDF |
| **2 weeks** | Oregon OEM "Be 2 Weeks Ready" | [paraphrase] A plan and supplies to survive **at least two weeks** after a large disaster | [paraphrase] It could take days or weeks for first responders to reach everyone. For a **Cascadia Subduction Zone M9+** earthquake and tsunami, Oregon expects residents to be without services and assistance for at least two weeks, if not longer. Coastal communities may be cut into isolated islands by landslides, liquefaction and bridge damage. Sewer and wastewater systems recover more slowly than drinking water. | https://www.oregon.gov/oem/hazardsprep/Pages/2-Weeks-Ready.aspx ; https://www.oregon.gov/oem/hazardsprep/pages/cascadia-subduction-zone.aspx ; https://www.oregon.gov/oem/hazardsprep/Pages/Individual-Preparedness.aspx |
| 2 weeks | Washington EMD "Prepare in a Year" | [paraphrase] One hour a month for 12 months to become 2 Weeks Ready; 14 gal of water per person | [paraphrase] After a large disaster it may take two weeks for resources to reach people | https://mil.wa.gov/prepare-in-a-year |
| 2 weeks | Church (water); CDC ("try"); Florida (medicines) | 14 gal/adult; 2 weeks of prescriptions | Disasters may pollute or disrupt water that long (Church) | §1.1, §3.2 |
| "several days or weeks" | Ready.gov Pandemic | Stay-home supplies | Community spread and staying home | https://www.ready.gov/pandemic |
| **1 month** | No major authority found that sets a household 1-month standard. | — | The Church manual's short-term ladder (1 week, then a month, and beyond) and the CDC pregnancy page's 30-day emergency refill are the nearest anchors. **Treat 1 month as an app-defined interpolation tier.** | — |
| **3 months** | Church, "three-month supply" (2007 pamphlet) | [paraphrase] A three-month supply of foods from your normal daily diet, built one week at a time and rotated | Supports self-reliance in adversity generally, including job loss and economic hardship, not only disasters. It pairs with a financial reserve. | §2.3 |
| 3-6 months (money) | FINRA; St. Louis Fed | 3-6 months of living or essential expenses | Job loss or a large financial setback | §8.3 |
| **1 year** | Church (historical "year's supply"; 2002 First Presidency letter via 2006 *Ensign*) and BYU 2019 list | 400/60/16/10 qt/60/8 basics (2006) or the BYU per-adult-year list | Store the basic foods that would keep you alive if you had nothing else to eat. Current policy (2007) instead describes a longer-term supply with no fixed duration. | §2.3 |

**Engine recommendation (DERIVED):**
- 72 h → evacuation / go-bag tier.
- 2 weeks → baseline home tier. This is where the Red Cross (home), Oregon, Washington and the CDC "try" all converge.
- 1 month → interpolation tier.
- 3 months → rotating everyday pantry (Church model).
- 1 year → staples-based long-term storage (Church/BYU), with the nutrient-gap warnings from USU.

---

## 12. COST BENCHMARKS (2025-2026 USD)

All prices observed 2026-09-25 unless a capture date is given. The DIY column is DERIVED.

| Item | Pre-made example (price) | DIY / unit basis | Source |
|---|---|---|---|
| **72-h kit for 4** | Red Cross Store *4-Person, 3-Day / 72-Hour Emergency Preparedness Kit* **$348.00** (Wayback 2026-04-18). Red Cross Deluxe 3-Day kit $184.00; Basic 3-Day kit $135.00; Basic 5-person 3-Day & Hygiene kit $149.95 (Wayback 2026-05-08). Ready America 72-Hour Deluxe 4-person kit: $57-$77 on Amazon per a search snippet (**UNVERIFIED**). My Patriot Supply "Family of 4 Starter Pack (1 Week)" $379.99. | DIY 72 h for 4, excluding items already owned: water 12 gal in 2 × 7-gal Aqua-Tainers ($47.98); food 12 person-days at TFP ≈ $101 (12 × $8.44); first-aid kit $34.95-$49.95; NOAA radio $54.99; flashlight, batteries, whistle, dust masks, sheeting, tape (**UNVERIFIED** prices). ≈ **$239-254** before unpriced items such as flashlight, batteries, whistle, masks, sheeting and tape (DERIVED, partial). | https://web.archive.org/web/20260418225514/https://www.redcross.org/store/4-person-3-day-emergency-preparedness-kit/91053.html ; https://web.archive.org/web/20260508193531/https://www.redcross.org/store/emergency-preparedness/72-hour-kits |
| **2-week food, 1 adult** | Mountain House 14-Day **$457.99** (~1,719 kcal/day). Ready Hour 4-week $277.95 (2,000+ kcal/day). | TFP: 14 × $8.44 = **$118** per person (reference family average). Everyday canned and dry goods are about the same order (**UNVERIFIED proxy**). | §2.7 |
| **2-week water, 1 person** | — | 14 gal: 2 × 7-gal Aqua-Tainer = **$47.98** ($3.43/gal). The 55-gal kit option is $2.36/gal on sale. | §1.5 |
| **1-month pantry, 1 adult** | Mountain House 30-Day $939.99; Augason 30-Day $182.99 (only 1,290 kcal/day); Ready Hour 4-week $277.95 | TFP: 30 × $8.44 = **$253** per person-month; Thrifty reference family of 4 = **$1,023.70/month** | §2.7 |
| **3-month supply, 1 adult** | Mountain House $2,749.99; Ready Hour $797.95; Augason $359.99 (1,290 kcal/day) | TFP about $760 (90 × $8.44) | §2.7 |
| **1-year long-term staples, 1 adult** | Mountain House 1-Year $10,366.99; Augason 1-Year $1,383.99 (1,290 kcal/day) | Church basics at Home Storage Center prices **≈$784** (+ oil and salt); BYU 2019 long-term list **≈$1,040** + short-term items | §2.7 |
| **Water storage per gallon** | — | $2.36-$3.43/gal (large drum or 7-gal jugs) up to $6-10/gal (small tanks) | §1.5 |
| **First-aid kit** | Red Cross 299-piece $34.95; Deluxe Family 115-piece $49.95; My Medic MyFAK $169.95 | Red Cross 19-item family-of-4 list: individual item prices **UNVERIFIED** | §3.1 |
| **NOAA weather radio** | Midland WR120 **$54.99**; WR400 $109.99; ER310 crank + power bank $89.99; ER10 $34.99 | — | §7 |
| **Small power station** | Jackery Explorer 1000 v2 (1,070 Wh / 1,500 W) **$559**; Westinghouse iGen400s $169 | $/Wh ≈ **$0.52/Wh** (Jackery, DERIVED) | §6.2 |
| **Generator** | Honda EU2200i **$1,199** (MSRP); Westinghouse iGen2550 **$499**; WGen3600DFv dual-fuel $499 | Fuel: 2.8-7.1 gal/day continuous for the EU2200i. Gasoline price **UNVERIFIED** (not captured). | §6.2 |
| **Water filter** | LifeStraw Family $54.95; Peak Gravity 8 L $95.95 | Bleach: price **UNVERIFIED** | §1.5 |
| **Two-way radios** | Midland GXT1000 GMRS $49.99 each; T71 FRS 2-pack $99.99. GMRS license $35 / 10 years. | — | §7 |
| **Twin-bucket toilet** | Reliance Luggable Loo $25.99; Double Doodie bags $16.99 | RDPO: buckets "a few dollars" each at hardware stores, or free from restaurants and bakeries | §4.1 |
| **DTC antibiotic kit** (reference only; do not recommend) | Typically **$300-400** per *Missouri Medicine* 2026 | — | §3.4 |
| **Emergency fund** | — | 3-6 months of expenses (FINRA; St. Louis Fed) | §8.3 |

---

## 13. WHERE AUTHORITIES DISAGREE (the engine must pick and disclose)

| Topic | Positions | Suggested handling |
|---|---|---|
| **Baseline duration** | FEMA/Ready.gov: "several days" (was 72 h until ~2021-22). CDC: 3 days, try 2 weeks. Red Cross: 3 days evacuation / 2 weeks home. Oregon/Washington: 2 weeks. Sweden: 1 week. Germany: 10 days. EU/Denmark: 72 h. Church: 3 months + longer-term. | Let the user choose, and default the home tier to 2 weeks with sources cited. |
| **Drinking share of water** | WA EMD: minimum 1 quart/day. Church: at least 2 quarts. Ready.gov: about 3/4 gal of fluid. Sphere/WHO: 2.5-3 L survival. Germany BBK: 2 L total, drinking plus cooking. Denmark: 3 L. | Keep 1 gal/p/d total (U.S. consensus) and show the ranges. |
| **Total water** | U.S.: 1 gal (3.8 L). Sphere: 15 L humanitarian household minimum (7.5 L briefly). WHO: 20 L for health and hygiene. | U.S. figures assume short outages and waterless hygiene (wipes, sanitizer). Flag Sphere/WHO as "long-duration or health-optimal". |
| **Boiling time and altitude** | CDC, EPA, Ready.gov: 1 minute. Church: 3-5 minutes. Altitude threshold for 3 minutes: **EPA 5,000 ft vs CDC 6,500 ft**. | Use 1 minute; use 3 minutes above 5,000 ft (the stricter threshold). |
| **Bleach dose table** | Ready.gov: 1/8 tsp/gal for 5.25-6% bleach (2021). CDC: 8 drops/gal for any 5-9% bleach, doubled if cloudy or cold. EPA: 8 drops (6%) vs 6 drops (8.25%). Church: 16 drops/gal if cloudy. | Ask for the bleach concentration and use the EPA table. Double for cloudy water (CDC). |
| **Water rotation** | CDC, Ready.gov, Church, WA: every 6 months (self-filled). ASPCA: every 2 months (pet kit bottled water). | Default 6 months; note the ASPCA figure. |
| **Fans in heat** | CDC: fans OK below 90°F indoors. Ready.gov: fans give a false sense of comfort and don't lower body temperature. | Warn above 90°F and prefer cooling centers. |
| **Kerosene heaters** | CDC lists them as a backup heat option (check local legality). IFC as cited by Lehi Fire: unvented portable kerosene heaters prohibited in occupied living spaces. | Show only as "if legal locally, vented/ventilated"; default off. |
| **Prescription reserve** | Red Cross 7 days; CDC 7-10 days (pregnancy page); CDC diabetes 1-2 weeks; Florida 2 weeks; state refill laws range from 72 h to 30 days (90 in NC). | Default 14 days on hand, plus an alert about emergency-refill rules in the user's state. |
| **Antibiotics** | CDC: don't save antibiotics for later. CDC Yellow Book: prescribed standby antibiotics for specific remote travel. WMS: prophylaxis only for open fractures and certain bites. DTC vendors: kits for when care is unavailable. *Mo Med* 2026: kits threaten stewardship. | No quantity; clinician card (§3.4). |
| **Nonfat dry milk shelf life** | Church web page: 20 years. BYU 2019 sheet and Home Storage Center form: 15 years. The HSC form also gives beans 25 (vs 30), oats 20 (vs 30), potato flakes 20 (vs 30). | Use the shorter (HSC/BYU) values. |
| **Church quantities** | 2006 *Ensign* basics (400 lb grain, 60 legumes, 16 milk, 10 qt oil, 60 sugar, 8 salt) vs BYU 2019 (~247 lb grains plus broader variety) vs official policy since 2007 (no quantities). | Offer the BYU 2019 list as the default (current, Church-linked) and the 2006 list as the minimal-cost option. |
| **Menstrual waste in the twin bucket** | RDPO: into the poo bucket. Oregon OEM: separate garbage bag. | Follow the local authority; default to a separate bag (Oregon) and note RDPO. |
| **CO deaths** | CDC basics page: >400/yr (non-fire). CDC PSA: >500/yr. | Cite ">400 non-fire deaths/yr (CDC)". |
| **Calories in kits vs needs** | Vendor "30-day" kits deliver 1,290-1,732 kcal/day. Sphere 2,100. DGA adult 1,600-3,000. | Normalize kits by kcal (§2.7). |

---

## 14. SUGGESTED ENGINE CONSTANTS (with source keys)

| Constant | Default | Range / alternatives | Source key |
|---|---|---|---|
| water_gal_per_person_day | 1.0 | 1.75-2.0 hot climate; Sphere 4.0 (15 L) "long-duration" option | Ready.gov water; CDC 2025; Sphere 2018 |
| water_lactation_add_L | +1.1 | — | DRI (IOM 2005) |
| water_pregnancy_add_L | +0.3 | — | DRI |
| water_formula_infant_L | 0.95 | powdered formula only | AAP; CDC |
| water_dog_oz_per_lb_day | 1.0 | Merck 132·kg^0.75 mL | PetMD; Merck |
| water_cat_oz_per_lb_day | 0.8 | Merck 80·kg^0.75 mL | PetMD; Merck |
| water_livestock_L_large | 25 | 20-30 | Sphere App. 3 |
| water_rehydration_gal_per_person_day (dehydrated diet) | 0.6 | — | DERIVED from ReadyWise spec |
| kcal_by_age_sex_activity | DGA Table A2-2 | Sphere 2,100 average for checks | DGA 2020-25; Sphere |
| kcal_pregnancy_add | T2 +340; T3 +452 | — | DGA A2-3 |
| kcal_lactation_add | +330 (0-6 mo); +400 (7-12 mo) | — | DGA A2-3 |
| food_cost_per_person_day_pantry | 8.44 | TFP by age and sex; household-size factors | USDA TFP Aug 2026 |
| longterm_staples_per_adult_year | BYU 2019 list | Church 2006 basics; child % 50/70/90/100 | BYU; Ensign 2006 |
| shelf_life_years | FSIS canned: low-acid 2-5, high-acid 1-1.5; staples per the HSC form | hot storage: grains 5 years | FSIS; Church; USU |
| rx_days_on_hand | 14 | 7-30 | CDC; Red Cross; Florida |
| antibiotic_qty | 0 (clinician card) | — | CDC; §3.4 |
| first_aid_kit | 1 per 4 persons (Red Cross list) | scale consumables per person | Red Cross |
| soap_bath_g_per_person_month | 250 | — | Sphere |
| soap_laundry_g_per_person_month | 200 | — | Sphere |
| menstrual_products | 2 cycles per menstruating person (≤1-month tiers) | Sphere 15 pads/month | CDC; Sphere |
| diapers_per_day | 8-12 (0-1 mo) … 6-8 (9-12 mo) | — | Pampers |
| toilet_bags_per_person_day | 0.3-0.6 (doubled-bag units) | ASSUMPTION | DERIVED from Oregon 5 gal/week, RDPO half-fill |
| cover_material_cups_per_person_day | 0.5-1.5 | ASSUMPTION | DERIVED from RDPO "handful" |
| fridge_Wh_day | 995 (top-freezer) | 537-1,753 by type | ENERGY STAR |
| cpap_Wh_night | 170 (humidified) / 96 (not) | AirMini 33-99 | SIL (user-measured) |
| solar_Wh_per_100W_day | from PVWatts by location, using the December value for winter sizing | 121-567 | PVWatts v8 (NLR) |
| generator_gal_per_day | 2.8 (¼ load) to 7.1 (rated) for a 2.2 kW inverter unit | — | Honda EU2200i spec, DERIVED |
| fuel_storage_limits | 25 gal flammable liquids (10 in attached garage, 0 indoors); propane ≤25 gal DOT capacity outdoors; ≤2 one-lb cylinders indoors | local codes vary | Lehi Fire (IFC/NFPA 58) |
| gas_tank_rule | ≥½ tank always; full when evacuation likely | — | Ready.gov Evacuation |
| cash | user-entered (days × daily spend) | no authority figure | Ready.gov; EFFAK |
| emergency_fund_months | 3-6 | CFPB: situation-dependent | FINRA; St. Louis Fed; CFPB |

---

## 15. METHOD NOTES AND FULL SOURCE LIST

**Method.**
- Retrieved 2026-09-25 with curl/pdftotext on primary documents where possible, plus WebFetch and Internet Archive captures for sites that block automated clients.
- 20 web searches were used, mainly to locate documents and a few vendor or state facts.
- Items marked [paraphrase] come from copyrighted or non-federal texts and are not quoted, so re-check wording before quoting them in the UI.
- Derived numbers show their formulas.

**Primary sources (by section):**

U.S. federal (public domain):
- Ready.gov (FEMA): https://www.ready.gov/water ; https://www.ready.gov/kit ; https://www.ready.gov/food ; https://www.ready.gov/pets ; https://www.ready.gov/disability ; https://www.ready.gov/older-adults ; https://www.ready.gov/evacuation ; https://www.ready.gov/shelter ; https://www.ready.gov/heat ; https://www.ready.gov/winter-weather ; https://www.ready.gov/power-outages ; https://www.ready.gov/financial-preparedness ; https://www.ready.gov/plan ; https://www.ready.gov/pandemic ; https://www.ready.gov/alerts ; https://www.ready.gov/low-and-no-cost ; https://www.ready.gov/earthquakes
- Ready.gov historical kit captures: https://web.archive.org/web/20200101090440/https://www.ready.gov/kit ; https://web.archive.org/web/20210701233712/https://www.ready.gov/kit ; https://web.archive.org/web/20220701233904/https://www.ready.gov/kit
- FEMA: EFFAK P-1075 https://www.ready.gov/sites/default/files/2020-03/ready_emergency-financial-first-aid-toolkit.pdf ; NFIP https://web.archive.org/web/2026/https://www.fema.gov/flood-insurance ; FloodSmart https://www.floodsmart.gov/get-insured/buy-a-policy
- CDC: water https://www.cdc.gov/water-emergency/about/how-to-create-and-store-an-emergency-water-supply.html and https://www.cdc.gov/water-emergency/about/index.html ; hygiene https://www.cdc.gov/water-emergency/safety/guidelines-for-personal-hygiene-during-an-emergency.html ; bleach https://www.cdc.gov/hygiene/about/cleaning-and-disinfecting-with-bleach.html ; CO https://www.cdc.gov/carbon-monoxide/about/index.html ; winter https://www.cdc.gov/winter-weather/safety/stay-safe-during-after-a-winter-storm-safety.html ; heat https://www.cdc.gov/heat-health/about/index.html ; NIOSH hydration https://www.cdc.gov/niosh/heat-stress/recommendations/index.html ; masks https://www.cdc.gov/respiratory-viruses/prevention/masks.html ; antibiotics https://www.cdc.gov/antibiotic-use/about/index.html ; Yellow Book https://www.cdc.gov/yellow-book/hcp/preparing-international-travelers/travel-health-kits.html , https://www.cdc.gov/yellow-book/hcp/preparing-international-travelers/travelers-diarrhea.html , https://www.cdc.gov/yellow-book/hcp/environmental-hazards-risks/heat-and-cold-illness-in-travelers.html ; pregnancy https://www.cdc.gov/reproductive-health/emergency-preparation-response/safety-messages.html ; infant feeding https://www.cdc.gov/breastfeeding/php/guidelines-recommendations/feeding-your-child-safely-during-a-disaster.html and https://www.cdc.gov/infant-feeding-emergencies-toolkit/php/checklist.html ; period factsheet https://www.cdc.gov/natural-disasters/media/pdfs/2026/09/FS-Managing-Period-During-Disaster-508.pdf ; menstrual hygiene https://www.cdc.gov/hygiene/about/menstrual-hygiene.html ; diabetes https://www.cdc.gov/diabetes/articles/diabetes-care-emergencies.html and https://www.cdc.gov/diabetes/articles/managing-insulin-in-emergency.html ; cholera/ORS https://www.cdc.gov/cholera/treatment/index.html ; evacuation PSA https://www.cdc.gov/natural-disasters/psa-toolkit/be-prepared-in-case-you-need-to-evacuate.html
- EPA: https://www.epa.gov/ground-water-and-drinking-water/emergency-disinfection-drinking-water ; ENERGY STAR datasets https://data.energystar.gov/resource/p5st-her9.json and https://data.energystar.gov/resource/8t9c-g3tn.json
- FDA: https://www.fda.gov/inspections-compliance-enforcement-and-criminal-investigations/warning-letters/chewy-inc-664707-11302023 ; https://www.fda.gov/drugs/special-features/dont-be-tempted-use-expired-medicines ; https://www.fda.gov/emergency-preparedness-and-response/mcm-legal-regulatory-and-policy-framework/expiration-dating-extension
- USDA: FSIS shelf-stable (Wayback 2026) https://www.fsis.usda.gov/food-safety/safe-food-handling-and-preparation/food-safety-basics/shelf-stable-food ; TFP https://www.fns.usda.gov/sites/default/files/resource-files/CostofFoodAug2026Thrifty.pdf ; three plans https://www.fns.usda.gov/sites/default/files/resource-files/CostofFoodAug2026LowModLib.pdf ; DGA 2020-2025 (Wayback) https://web.archive.org/web/20250101015914id_/https://www.dietaryguidelines.gov/sites/default/files/2020-12/Dietary_Guidelines_for_Americans_2020-2025.pdf
- HHS/ASPR EPAP: https://aspr.hhs.gov/EPAP/Pages/default.aspx
- CFPB: https://www.consumerfinance.gov/an-essential-guide-to-building-an-emergency-fund/
- DOE Energy Saver (Wayback): https://web.archive.org/web/20241231180235/https://www.energy.gov/energysaver/estimating-appliance-and-home-electronic-energy-use
- EIA: https://www.eia.gov/energyexplained/units-and-calculators/british-thermal-units.php
- NLR/NREL PVWatts v8: https://developer.nlr.gov/api/pvwatts/v8.json
- NWS NWR: https://www.weather.gov/nwr/
- FCC (Wayback 2026): GMRS, FRS, WEA and amateur pages as linked in §7
- eCFR: 47 CFR 10.430 https://www.ecfr.gov/current/title-47/section-10.430 ; 47 CFR 1.1102 https://www.ecfr.gov/current/title-47/section-1.1102 ; 49 CFR 180.209 https://www.ecfr.gov/current/title-49/section-180.209
- U.S. Code 21 USC 353: https://www.law.cornell.edu/uscode/text/21/353
- FHWA MUTCD 2009 §4E.06: https://mutcd.fhwa.dot.gov/htm/2009/part4/part4e.htm

Humanitarian and international:
- Sphere Handbook 2018 (ReliefWeb mirror): https://reliefweb.int/attachments/fc910c0d-e277-3f9e-930e-7c1dfc8182e8/Sphere-Handbook-2018-EN.pdf
- WHO/WEDC Technical Note 9: https://cdn.who.int/media/docs/default-source/wash-documents/who-tn-09-how-much-water-is-needed.pdf
- National Academies DRI water (Wayback NAP): https://web.archive.org/web/2019/https://www.nap.edu/read/10925/chapter/6
- EU JOIN(2025) 130: https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX:52025JC0130
- Germany BBK: https://www.bbk.bund.de/SharedDocs/Downloads/DE/Mediathek/Publikationen/Buergerinformationen/Ratgeber/ratgeber-notfallvorsorge.pdf?__blob=publicationFile
- Denmark BRS: https://www.brs.dk/en/prepared/
- Sweden MSB: https://www.msb.se/sv/amnesomraden/msbs-arbete-vid-olyckor-kriser-och-krig/om-krisen-eller-kriget-kommer/

States and localities:
- Oregon OEM: https://www.oregon.gov/oem/hazardsprep/Pages/2-Weeks-Ready.aspx ; toolkit https://www.oregon.gov/oem/Documents/B2WR-Complete-Tool-Kit-EN.pdf ; Cascadia https://www.oregon.gov/oem/hazardsprep/pages/cascadia-subduction-zone.aspx
- Washington EMD: https://mil.wa.gov/prepare-in-a-year (guide https://mil.wa.gov/asset/5f171cc0a935f)
- Florida DEM: https://www.floridadisaster.org/planprepare/disability/personal-and-family-plans/medication/
- RDPO (Portland): https://www.rdpo.net/emergency-toilet
- Lehi City Fire: https://www.lehi-ut.gov/departments/fire-department/fire-prevention/home-fuel-storage-limits/

Church, BYU and USU:
- Church: https://www.churchofjesuschrist.org/bc/content/shared/content/english/pdf/language-materials/04008_eng.pdf ; manual pages as linked in §2.3 ; Ensign 2006 https://www.churchofjesuschrist.org/study/ensign/2006/03/random-sampler/food-storage-for-one-year?lang=eng ; HSC order form https://assets.churchofjesuschrist.org/01/b4/01b41d0ffdbf11ebbb5eeeeeac1e9b2afdc29be5/food_storage_center_products.pdf ; HSC prices page https://www.churchofjesuschrist.org/life/home-storage-centers/prices-locations?lang=eng
- BYU 2019: https://brightspotcdn.byu.edu/b1/4d/75fc449e4ce9843daa701f69faa4/an-approach-to-longer-term-food-storage.SEPT2019.pdf
- USU Extension Food Storage Booklet (©2013): https://extension.usu.edu/preserve-the-harvest/files/Food-Storage-Booklet.pdf
- Deseret News 2002: https://www.deseret.com/2002/2/20/19638826/lds-stress-storage-savings/

NGOs, clinical and journals:
- Red Cross (Wayback): first-aid kit and survival-kit pages as linked in §1.1 and §3.1 ; store captures as linked in §12
- ASPCA: https://www.aspca.org/pet-care/general-pet-care/disaster-preparedness
- AAP HealthyChildren: https://www.healthychildren.org/English/ages-stages/baby/formula-feeding/Pages/Amount-and-Schedule-of-Formula-Feedings.aspx
- Merck Vet Manual: https://www.merckvetmanual.com/therapeutics/fluid-therapy/maintenance-fluid-plan-in-animals
- PetMD: https://www.petmd.com/dog/nutrition/evr_dg_the_importance_of_water
- WMS 2014: https://doi.org/10.1016/j.wem.2014.08.015
- PLoS One 2020: https://pmc.ncbi.nlm.nih.gov/articles/PMC7470343/
- Mo Med 2026: https://pmc.ncbi.nlm.nih.gov/articles/PMC13585011/
- Healthcare Ready 2022: https://healthcareready.org/a-review-of-state-emergency-prescription-protocols/
- SIL CPAP: https://power.sil.org/cpap-power-options/
- FINRA: https://www.finra.org/investors/investing/investing-basics/financial-foundations
- St. Louis Fed: https://www.stlouisfed.org/publications/page-one-economics/2025/sep/when-unexpected-happens-be-ready-with-emergency-fund
- Meshtastic docs: as linked in §7
- Pampers: https://www.pampers.com/en-us/baby/diapering/article/how-many-diapers-a-day
- Sutter Health: https://www.sutterhealth.org/health/how-many-diapers-a-day-your-newborn-should-have

Vendors (prices; volatile):
- relianceoutdoors.com, lifestraw.com, mountainhouse.com, augasonfarms.com, readywise.com, mypatriotsupply.com, midlandusa.com and westinghouseoutdoorpower.com: `/products.json` endpoints
- Jackery product page; Anker product page; Honda EU2200i (Wayback 2026-09-14); Church online store
- Jase Medical: https://jasemedical.com/
- Duration Health (unresolvable from this environment): https://durationhealth.com/kit/1/about

**Open items (UNVERIFIED, worth a follow-up pass):**
- FEMA *Food and Water in an Emergency* text
- IDSA statement on stockpiles
- Georgia and Texas refill statutes and the Kevin's Law state list at primary source
- Gasoline shelf life from a manufacturer or regulator
- Propane "no degradation" statement
- Well-pump and oxygen-concentrator wattage from manufacturers
- Smartphone Wh
- Bottled-water, bleach and gasoline retail prices
- VE exam fee; Meshtastic node prices
- Toddler diapers per day
- Home sugar-salt ORS recipe
- Cold-climate kcal increment
- Plastic-sheeting thickness
- Sphere 5th-edition timing
