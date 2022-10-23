pub struct Flatten<I>
where
    I: IntoIterator,
    I::Item: IntoIterator,
{
    iter: I::IntoIter,
    front_iter: Option<<I::Item as IntoIterator>::IntoIter>,
    back_iter: Option<<I::Item as IntoIterator>::IntoIter>,
}

impl<I> Flatten<I>
where
    I: IntoIterator,
    I::Item: IntoIterator,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter: iter.into_iter(),
            front_iter: None,
            back_iter: None,
        }
    }
}

impl<I> Iterator for Flatten<I>
where
    I: IntoIterator,
    I::Item: IntoIterator,
{
    type Item = <I::Item as IntoIterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.front_iter.as_mut() {
                None => match self.iter.next() {
                    None => return self.back_iter.as_mut()?.next(), // None
                    Some(sub_iter) => self.front_iter = Some(sub_iter.into_iter()),
                },
                Some(sub_iter) => match sub_iter.next() {
                    None => self.front_iter = None,
                    Some(item) => return Some(item),
                },
            }
        }
        //self.iter.next().and_then(|inner| inner.into_iter().next())
    }
}

impl<I> DoubleEndedIterator for Flatten<I>
where
    I: IntoIterator,
    I::IntoIter: DoubleEndedIterator,
    I::Item: IntoIterator,
    <I::Item as IntoIterator>::IntoIter: DoubleEndedIterator,
{
    // type Item = <I::Item as IntoIterator>::Item; // use super trait Iterator's
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            match self.back_iter.as_mut() {
                None => match self.iter.next_back() {
                    None => return self.front_iter.as_mut()?.next_back(), //None,
                    Some(sub_iter) => self.back_iter = Some(sub_iter.into_iter()),
                },
                Some(sub_iter) => match sub_iter.next_back() {
                    None => self.back_iter = None,
                    Some(item) => return Some(item),
                },
            }
        }
        //self.iter.next().and_then(|inner| inner.into_iter().next())
    }
}

trait IteratorFlatten: IntoIterator
where
    Self: Sized,
    Self::Item: IntoIterator,
{
    fn flatten(self) -> Flatten<Self> {
        Flatten::new(self)
    }
}

impl IteratorFlatten for Vec<Vec<i32>> {}

fn main() {
    //let mut iter = Flatten::new(vec![vec![1, 2], vec![], vec![4, 5, 6]]);
    //let mut iter = Flatten::new(vec![vec![1, 2, 3], vec![], vec![5, 6]]);
    let mut iter = vec![vec![1, 2, 3], vec![], vec![5, 6]].flatten();
    assert_eq!(Some(1), iter.next());
    assert_eq!(Some(6), iter.next_back());
    assert_eq!(Some(2), iter.next());
    assert_eq!(Some(5), iter.next_back());
    //assert_eq!(Some(4), iter.next());
    assert_eq!(Some(3), iter.next_back());
    assert_eq!(None, iter.next());
    assert_eq!(None, iter.next_back());
}
