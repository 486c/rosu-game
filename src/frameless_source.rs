use std::time::Duration;

use rodio::{source::SeekError, Sample, Source};

pub struct FramelessSource<I>
where
  I: Source,
  I::Item: Sample,
{
  inner: I,
}

impl<I> FramelessSource<I>
where
  I: Source,
  I::Item: Sample,
{
  pub fn new(source: I) -> Self {
    Self { inner: source }
  }
}

impl<I> From<I> for FramelessSource<I>
where
  I: Source,
  I::Item: Sample,
{
  fn from(value: I) -> Self {
    Self::new(value)
  }
}

impl<I> Iterator for FramelessSource<I>
where
  I: Source,
  I::Item: Sample,
{
  type Item = I::Item;

  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next()
  }
}

impl<I> Source for FramelessSource<I>
where
  I: Source,
  I::Item: Sample,
{
  fn current_frame_len(&self) -> Option<usize> {
    None
  }

  fn channels(&self) -> u16 {
    self.inner.channels()
  }

  fn sample_rate(&self) -> u32 {
    self.inner.sample_rate()
  }

  fn total_duration(&self) -> Option<std::time::Duration> {
    self.inner.total_duration()
  }

  fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
      self.inner.try_seek(pos)
  }
}
