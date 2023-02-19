use std::ops::{Deref, DerefMut};

pub struct Pin<P> {
    pointer: P,
    // std 里这里用了 pub, 但我们自己要用的话得加 #!feature[unsafe_pin_internals]
}

impl<P: Deref> Pin<P> {
    pub const unsafe fn new_unchecked(pointer: P) -> Self {
        Self { pointer }
    }

    pub fn as_ref(&self) -> Pin<&<P as Deref>::Target> {
        unsafe { Pin::new_unchecked(&*self.pointer) }
    }
}

impl<P: DerefMut> Pin<P> {
    pub fn as_mut(&mut self) -> Pin<&mut <P as Deref>::Target> {
        unsafe { Pin::new_unchecked(&mut *self.pointer) }
    }
}

impl<'a, T: ?Sized> Pin<&'a T> {
    pub const fn get_ref(self) -> &'a T {
        self.pointer
    }
}

impl<'a, T: ?Sized + Unpin> Pin<&'a mut T> {
    pub fn get_mut(self) -> &'a mut T {
        self.pointer // T: Unpin
    }

    pub unsafe fn get_unchecked_mut(self) -> &'a mut T {
        self.pointer // unsafe
    }
}

impl<P: Deref> Deref for Pin<P> {
    type Target = <P as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        Pin::get_ref(Pin::as_ref(self))
    }
}

impl<P: DerefMut> DerefMut for Pin<P>
where
    <P as Deref>::Target: Unpin,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        Pin::get_mut(Pin::as_mut(self))
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomPinned;

    // test example from:
    // https://rust-lang.github.io/async-book/04_pinning/01_chapter.html

    #[test]
    fn test_pin_not_change_receiver() {
        use super::*;

        #[derive(Debug)]
        struct Test {
            a: String,
            b: *const String,
            //_marker: PhantomPinned, // commit it or not
        }

        impl Test {
            fn new(txt: &str) -> Self {
                Self {
                    a: String::from(txt),
                    b: std::ptr::null(),
                    //_marker: PhantomPinned,
                }
            }

            fn init(&mut self) {
                let self_ref: *const String = &self.a;
                self.b = self_ref;
            }

            fn a(&self) -> &str {
                &self.a
            }

            fn b(&self) -> &String {
                unsafe { &*(self.b) }
            }
        }

        let mut test1 = Test::new("test1");
        // 这里的 shadowing 太棒了, 不会释放原来而空间, 又能隐藏入口
        let mut test1 = unsafe { Pin::new_unchecked(&mut test1) };
        test1.init();
        let mut test2 = Test::new("test2");
        let mut test2 = unsafe { Pin::new_unchecked(&mut test2) };
        test2.init();

        // use deref
        println!("a: {}, b: {}", test1.a(), test1.b());
        // std::mem::swap(&mut test1, &mut test2); // no ok
        std::mem::swap(test1.as_mut().get_mut(), test2.as_mut().get_mut());
        //std::mem::swap(Pin::get_mut(Pin::as_mut(&mut test1)), Pin::get_mut(Pin::as_mut(&mut test2))); // 正规写法但是太啰嗦
        println!("a: {}, b: {}", test2.a(), test2.b());
    }

    #[test]
    fn test_pin_book() {
        use std::pin::Pin;

        #[derive(Debug)]
        struct Test {
            a: String,
            b: *const String,
            //_marker: PhantomPinned, // commit it or not
        }

        impl Test {
            fn new(txt: &str) -> Self {
                Self {
                    a: String::from(txt),
                    b: std::ptr::null(),
                    //_marker: PhantomPinned,
                }
            }

            fn init(self: Pin<&mut Self>) {
		// 要作为 receiver 的话, 自己实现的 Pin 是不行的
                let self_ref: *const String = &self.a;
                unsafe {
                    self.get_unchecked_mut().b = self_ref;
                }
            }

            fn a(self: Pin<&Self>) -> &str {
                &self.get_ref().a
            }

            fn b(self: Pin<&Self>) -> &String {
                assert!(
                    !self.b.is_null(),
                    "Test::b called without Test::init being called first"
                );
                unsafe { &*(self.get_ref().b) }
            }
        }

        let mut test1 = Test::new("test1");
        let mut test1 = unsafe { Pin::new_unchecked(&mut test1) };
        test1.as_mut().init();
        let mut test2 = Test::new("test2");
        let mut test2 = unsafe { Pin::new_unchecked(&mut test2) };
        test2.as_mut().init();

        println!(
            "a: {}, b: {}",
            Test::a(test1.as_ref()),
            Test::b(test1.as_ref())
        );
        std::mem::swap(test1.as_mut().get_mut(), test2.as_mut().get_mut());
        println!(
            "a: {}, b: {}",
            Test::a(test2.as_ref()),
            Test::b(test2.as_ref())
        );
    }
}
