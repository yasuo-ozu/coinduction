use coinduction::traitdef;

pub struct TestMarker;
pub struct LocalMarker;
pub struct CoinductionTestMarker;

// Local trait that exists only in this crate
#[traitdef]
pub trait LocalTrait {
    fn local_method(&self) -> usize;
}

#[traitdef(marker = TestMarker)]
pub trait TestTrait {
    fn test_method(&self) -> String;
}

#[traitdef(marker = LocalMarker)]  
pub trait CircularTrait {
    fn circular_method(&self) -> Box<dyn CircularTrait>;
}

#[traitdef(marker = CoinductionTestMarker)]
pub trait ExtendedTrait {
    fn extended_method(&self) -> bool;
}