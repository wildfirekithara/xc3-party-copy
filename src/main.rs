use std::num::NonZeroUsize;
use recordkeeper::item::edit::ItemEditor;
use strum::IntoEnumIterator;

fn name_list(input: &'static str) -> Vec<&'static str> {
    input.split('\n').collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        println!("usage: {} <base save file (bf3game0X.sav)> <dlc save file (bf3dlc0X.sav)> <output save file>", args[0]);
        return;
    }

    let base_in = &args[1];
    let dlc_in = &args[2];
    let base_out = &args[3];

    let character_names = name_list(include_str!("../data/pcnames.txt"));
    let class_names = name_list(include_str!("../data/talentnames.txt"));
    let art_names = name_list(include_str!("../data/artnames.txt"));
    let skill_names = name_list(include_str!("../data/skillnames.txt"));
    let gem_names = name_list(include_str!("../data/gemnames.txt"));
    // let gem_names_dlc = name_list(include_str!("../data/gemnames4.txt"));
    let accessory_names = name_list(include_str!("../data/accnames.txt"));

    let base_bytes = std::fs::read(base_in).unwrap();
    let dlc_bytes = std::fs::read(dlc_in).unwrap();

    let mut base_savefile = recordkeeper::SaveFile::from_bytes(&base_bytes).unwrap();
    let dlc_savefile = recordkeeper::SaveFile::from_bytes(&dlc_bytes).unwrap();

    let base_save = base_savefile.save_mut();
    let dlc_save = dlc_savefile.save();

    println!("Current base game party:");

    for (i, idx) in base_save.party_characters.iter().enumerate() {
        if *idx == 0 {
            continue;
        }
        println!("{}: {} ({:02X})", i + 1, character_names[*idx as usize], idx);
    }    

    println!("\nDLC party:");

    for (i, idx) in dlc_save.party_characters.iter().enumerate() {
        if *idx == 0 {
            continue;
        }

        let charid = *idx as usize;
        let charslotid = charid - 1;
        println!("{}: {} ({:02X})", i + 1, character_names[charid], charid);

        let character = dlc_save.characters[charslotid];
        let mut base_character = base_save.characters[charslotid];
        
        println!("  level: {}", character.level);
        base_character.level = character.level;        
        println!("  exp: {}", character.exp);
        base_character.exp = character.exp;

        println!("  flags:");
        for cf in recordkeeper::character::CharacterFlag::iter() {
            println!("    {}", character.is_flag_set(cf));
            base_character.set_flag(cf, character.is_flag_set(cf));
        }

        println!("  is selectable: {}", dlc_save.character_sets.selectable_characters.get(charslotid).unwrap());
        base_save.character_sets.selectable_characters.set(charslotid, dlc_save.character_sets.selectable_characters.get(charslotid).unwrap());

        println!("  is permanent: {}", dlc_save.character_sets.permanent_characters.get(charslotid).unwrap());
        base_save.character_sets.permanent_characters.set(charslotid, dlc_save.character_sets.permanent_characters.get(charslotid).unwrap());

        println!("  is temporary: {}", dlc_save.character_sets.temporary_characters.get(charslotid).unwrap());
        base_save.character_sets.temporary_characters.set(charslotid, dlc_save.character_sets.temporary_characters.get(charslotid).unwrap());


        println!("  class: {}", class_names[character.selected_class as usize]);
        base_character.selected_class = character.selected_class;

        println!("\n  class data:");
        let class_data = character.class_data(character.selected_class as usize);
        let base_class_data = base_character.class_data_mut(character.selected_class as usize);

        println!("  cp: {}", class_data.cp);
        base_class_data.cp = class_data.cp;
        println!("  rank: {}", class_data.level);
        base_class_data.level = class_data.level;
        println!("  unlock points: {}", class_data.unlock_points);
        base_class_data.unlock_points = class_data.unlock_points;

        println!("  arts:");        
        class_data.arts().enumerate().for_each(|(i, art)| {
            if let Some(art) = art.get() {
                println!("    {}: {}", i + 1, art_names[art as usize]);
                base_class_data.art_slot_mut(i).set(Some(art));
            }
        });

        println!("  skills:");
        class_data.skills().enumerate().for_each(|(i, skill)| {
            if let Some(skill) = skill.get() {
                println!("    {}: {}", i + 1, skill_names[skill as usize]);
                base_class_data.skill_slot_mut(i).set(Some(skill));
            }
        });

        println!("  gems:");
        class_data.gems().enumerate().for_each(|(i, gem)| {
            if let Some(gem) = gem.get() {
                println!("    {}: {}", i + 1, gem_names[(gem as usize) * 10]);
                base_class_data.gem_slot_mut(i).set(Some(gem));
            }
        });

        println!("  accessories:");
        class_data.accessories().enumerate().for_each(|(acc_idx, acc)| {
            if let Some(acc) = acc.get() {
                println!("    {}: {}", i + 1, accessory_names[acc.bdat_id() as usize]);

                // add the accessory to the base game inventory first
                let mut free_idx = None;
                for (i, slot) in base_save.inventory.slots(recordkeeper::item::ItemType::Accessory).iter().enumerate() {
                    if slot.item_id() == 0 {
                        free_idx = Some(i);
                        break;
                    }
                }
                if free_idx.is_none() {
                    panic!("no free accessory slots");
                }
                let free_idx = free_idx.unwrap();

                let mut editor = ItemEditor::new(base_save, recordkeeper::item::ItemType::Accessory, free_idx);
                editor.set_item_id(acc.bdat_id()).unwrap();
                let item_slot = base_save.inventory.slots(recordkeeper::item::ItemType::Accessory)[free_idx];

                // now we can set the accessory onto the character class
                base_class_data.accessory_slot_mut(acc_idx).set_from_inventory(&item_slot);
            }

            // affinity growth
            let mut dlc_pow_aug = None;
            for (_, pa) in dlc_save.pow_augment.iter().enumerate() {
                if pa.chr_id as usize == charid as usize {
                    dlc_pow_aug = Some(pa);
                    break;
                }
            }
            if dlc_pow_aug.is_none() {
                panic!("no pow aug in dlc save found for character {}", charid);
            }
            let dlc_pow_aug = dlc_pow_aug.unwrap();

            let mut base_pow_aug = None;
            for (_, pa) in base_save.pow_augment.iter_mut().enumerate() {
                if pa.chr_id as usize == charid as usize {
                    base_pow_aug = Some(pa);
                    break;
                }
            }
            if base_pow_aug.is_none() {
                panic!("no pow aug in base save found for character {}", charid);
            }
            let base_pow_aug = base_pow_aug.unwrap();

            base_pow_aug.unlocked_tiers = dlc_pow_aug.unlocked_tiers;
            for i in 1..=64 {
                let i = NonZeroUsize::new(i).unwrap();
                base_pow_aug.set_learned(i, dlc_pow_aug.is_learned(i));
            }
        });

        base_save.characters[charslotid] = base_character;
        base_save.party_characters.set(i, charid as u16);
    }

    base_savefile.write().unwrap();

    let new_bytes = base_savefile.bytes();

    std::fs::write(base_out, new_bytes).unwrap();
}
