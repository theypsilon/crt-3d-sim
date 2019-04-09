use enum_len_trait::EnumLen;
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Copy, Clone, Default)]
pub struct Size2D<T: Copy + Clone + Default> {
    pub width: T,
    pub height: T,
}

pub trait NextEnumVariant {
    fn next_enum_variant(&mut self);
    fn previous_enum_variant(&mut self);
}

impl<T> NextEnumVariant for T
where
    T: FromPrimitive + ToPrimitive + EnumLen,
{
    fn next_enum_variant(&mut self)
    where
        Self: FromPrimitive + ToPrimitive,
    {
        change_enum_variant(self, |u| u + 1)
    }

    fn previous_enum_variant(&mut self)
    where
        Self: FromPrimitive + ToPrimitive,
    {
        change_enum_variant(self, |u| if u == 0 { Self::len() - 1 } else { u - 1 })
    }
}

fn change_enum_variant<T: FromPrimitive + ToPrimitive + EnumLen>(instance: &mut T, action: impl Fn(usize) -> usize) {
    let mut changed = match instance.to_usize().and_then(|as_usize| FromPrimitive::from_usize(action(as_usize))) {
        Some(n) => n,
        None => FromPrimitive::from_usize(0).expect("Can't construct enum from 0."),
    };
    std::mem::swap(instance, &mut changed);
}

pub fn f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

pub fn i32_to_u8(v: &[i32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

pub fn transform_u32_to_array_of_u8(x: u32) -> [u8; 4] {
    let b1: u8 = ((x >> 24) & 0xff) as u8;
    let b2: u8 = ((x >> 16) & 0xff) as u8;
    let b3: u8 = ((x >> 8) & 0xff) as u8;
    let b4: u8 = (x & 0xff) as u8;
    [b1, b2, b3, b4]
}

pub fn get_3_f32color_from_int(color: i32) -> [f32; 3] {
    [
        (color >> 16) as f32 / 255.0,
        ((color >> 8) & 0xFF) as f32 / 255.0,
        (color & 0xFF) as f32 / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    mod get_3_f32color_from_int {
        mod gives_good {
            use super::super::super::*;

            macro_rules! get_3_f32color_from_int_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, expected) = $value;
                assert_eq!(expected, get_3_f32color_from_int(input));
            }
        )*
        }
    }

            get_3_f32color_from_int_tests! {
                white: (0x00FF_FFFF, [1.0, 1.0, 1.0]),
                black: (0x0000_0000, [0.0, 0.0, 0.0]),
                red: (0x00FF_0000, [1.0, 0.0, 0.0]),
                green: (0x0000_FF00, [0.0, 1.0, 0.0]),
                blue: (0x0000_00FF, [0.0, 0.0, 1.0]),
                yellow: (0x00eb_f114, [0.92156863, 0.94509804, 0.078431375]),
            }
        }
    }
}
