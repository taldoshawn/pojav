//! Legacy inventory slots retain numeric IDs, damage and named-root NBT.
use crate::{
    Error, Result,
    buf::{NBT_QUOTA, Reader, Writer},
};
use horizon_world::nbt::{self, Tag};
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub id: i16,
    pub count: i8,
    pub damage: i16,
    pub tag: Tag,
}
impl Slot {
    pub fn read(r: &mut Reader<'_>) -> Result<Option<Self>> {
        let id = r.i16()?;
        if id == -1 {
            return Ok(None);
        }
        if id < 0 {
            return Err(Error::new("invalid legacy item ID"));
        }
        let count = r.i8()?;
        let damage = r.i16()?;
        let tag = r.named_nbt()?;
        if !matches!(tag, Tag::End | Tag::Compound(_)) {
            return Err(Error::new("legacy item NBT must be a compound"));
        }
        if count <= 0 {
            return Ok(None);
        }
        Ok(Some(Self {
            id,
            count,
            damage,
            tag,
        }))
    }
    pub fn write(slot: Option<&Self>, w: &mut Writer) -> Result<()> {
        let Some(slot) = slot else {
            w.i16(-1);
            return Ok(());
        };
        if slot.id < 0 || slot.count <= 0 {
            return Err(Error::new("invalid legacy slot"));
        }
        w.i16(slot.id);
        w.i8(slot.count);
        w.i16(slot.damage);
        let root = nbt::NamedTag {
            name: "".into(),
            tag: slot.tag.clone(),
        };
        let bytes = nbt::write_named(
            &root,
            nbt::Limits {
                max_bytes: NBT_QUOTA,
                ..Default::default()
            },
        )
        .map_err(|e| Error::new(e.to_string()))?;
        w.bytes(&bytes);
        Ok(())
    }
    /// Upgrade into the current item representation without losing legacy NBT.
    pub fn upgraded(&self) -> Tag {
        let mut item = Tag::compound([
            ("id", Tag::Short(self.id)),
            ("Count", Tag::Byte(self.count)),
            ("Damage", Tag::Short(self.damage)),
            (
                "tag",
                if self.tag == Tag::End {
                    Tag::Compound(Default::default())
                } else {
                    self.tag.clone()
                },
            ),
        ]);
        horizon_world::datafix::update(
            horizon_world::datafix::schema::Ty::ItemStack,
            &mut item,
            99,
            horizon_world::datafix::CURRENT,
        );
        item
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_and_named_tag_slots_preserve_packet_boundaries() {
        let bytes = [0xff, 0xff, 0, 1, 64, 0, 0, 10, 0, 0, 0, 99];
        let mut r = Reader::new(&bytes);
        assert_eq!(Slot::read(&mut r).unwrap(), None);
        let slot = Slot::read(&mut r).unwrap().unwrap();
        assert_eq!((slot.id, slot.count, slot.damage), (1, 64, 0));
        assert_eq!(slot.tag, Tag::Compound(Default::default()));
        assert_eq!(r.u8().unwrap(), 99);
        let mut w = Writer::new();
        Slot::write(Some(&slot), &mut w).unwrap();
        assert_eq!(w.data, &bytes[2..11]);
        assert!(Slot::read(&mut Reader::new(&bytes[2..10])).is_err());
    }
    #[test]
    fn legacy_item_ids_and_damage_upgrade_to_current_items() {
        let slot = Slot {
            id: 35,
            count: 12,
            damage: 14,
            tag: Tag::End,
        };
        let item = slot.upgraded();
        assert_eq!(
            item.get("id"),
            Some(&Tag::String("minecraft:red_wool".into()))
        );
        assert_eq!(item.get("count"), Some(&Tag::Int(12)));
    }
}
