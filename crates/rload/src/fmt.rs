use std::time::Duration;

pub struct FormatDuration(pub Duration);

pub fn format_duration(d: Duration) -> FormatDuration {
  FormatDuration(d)
}

impl std::fmt::Display for FormatDuration {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    
    let dur = self.0;

    let mut formatter = numfmt::Formatter::new()
      .comma(false)
      .precision(numfmt::Precision::Significance(4));

    if dur < Duration::from_micros(1) {
      write!(f, "{}ns", dur.as_nanos())?;
    } else if dur < Duration::from_millis(1) {
      write!(f, "{}us", formatter.fmt2(dur.as_nanos() as f64 / 1_000.0))?;
    } else if dur < Duration::from_secs(1) {
      write!(f, "{}ms", formatter.fmt2(dur.as_micros() as f64 / 1_000.0))?;
    } else if dur < Duration::from_secs(60)  {
      write!(f, "{}s", formatter.fmt2(dur.as_secs_f64()))?;
    } else if dur < Duration::from_secs(60 * 60) {
      let all = dur.as_secs_f64();
      let m = (all / 60.0).floor() as u16;
      let s = (all % 60.0).round() as u16;
      if s == 0 {
        write!(f, "{}m", m)?;
      } else {
        write!(f, "{}m {}s", m, s)?;
      }
    } else if dur < Duration::from_secs(60 * 60 * 24) {
      let all = dur.as_secs_f64();
      let h = (all / 60.0 / 60.0).floor() as u16;
      let m = (all / 60.0).floor() as u16;
      let s = (all % 60.0).round() as u16;
      if s != 0 {
        write!(f, "{}h {}m {}s", h, m, s)?;
      } else if m != 0 {
        write!(f, "{}h {}m", h, m)?;
      } else {
        write!(f, "{}h", h)?;
      }
    } else {
      let all = dur.as_secs_f64();
      let d = (all / 60.0 / 60.0 / 24.0).floor() as u16;
      let h = (all / 60.0 / 60.0).floor() as u16;
      let m = (all / 60.0).floor() as u16;
      let s = (all % 60.0).round() as u16;
      if s != 0 {
        write!(f, "{}d {}h {}m {}s", d, h, m, s)?;
      } else if m != 0 {
        write!(f, "{}d {}h {}m", d, h, m)?;
      } else if h != 0 {
        write!(f, "{}d {}h", d, h)?;
      } else {
        write!(f, "{}d", d)?;
      }
    }


    Ok(())
  }
}

