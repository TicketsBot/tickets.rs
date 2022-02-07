#[inline(always)]
pub fn replace_if_some<T>(dest: &mut T, src: Option<T>) {
    if let Some(t) = src {
        *dest = t;
    }
}

#[inline(always)]
pub fn replace_option_if_some<T>(dest: &mut Option<T>, src: Option<T>) {
    if let Some(t) = src {
        *dest = Some(t);
    }
}
