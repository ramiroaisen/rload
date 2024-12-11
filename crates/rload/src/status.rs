// a status-codes counter for status codes in the ranges of 0..=999
#[derive(Debug, Clone)]
pub struct Statuses([u64; 1000]);

#[derive(Debug)]
pub struct StatusOutOfRangeError {
  pub status: u16,
}

impl std::error::Error for StatusOutOfRangeError {}

impl std::fmt::Display for StatusOutOfRangeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "status code {} is out of range (greater than 999)", self.status)
  }
}

impl Statuses {
  #[inline(always)]
  pub fn new() -> Self {
    Self([0; 1000])
  }

  /// # Safety
  /// the caller must ensure that the status is <= 999
  #[inline(always)]
  pub unsafe fn record_unchecked(&mut self, status: u16) {
    *self.0.get_unchecked_mut(status as usize) += 1;
  }


  /// Errors if the status is greater than 999
  #[inline(always)]
  pub fn record(&mut self, status: u16) -> Result<(), StatusOutOfRangeError> {
    if status <= 999 {
      unsafe { self.record_unchecked(status) };
      Ok(())
    } else {
      Err(StatusOutOfRangeError { status })
    }
  }

  #[inline(always)]
  pub fn join(&mut self, other: Self) {
    for (i, v) in other.0.iter().enumerate() {
      // Safety: the length of both arrays is always the same
      unsafe {
        *self.0.get_unchecked_mut(i) += *v;
      }
    }
  }

  /// An iterator over the non-zero-count pairs of (status: u16, count: u64)
  pub fn iter(&self) -> impl Iterator<Item = (u16, u64)> + '_ {
    self.0.iter().enumerate()
      .filter_map(|(status, count)| {
        if *count == 0 {
          None
        } else {
          Some((status as u16, *count))
        }
      })
  }
}

impl Default for Statuses {
  #[inline(always)]
  fn default() -> Self {
    Self::new()
  }
}