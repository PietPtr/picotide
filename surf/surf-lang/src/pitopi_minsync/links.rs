pub trait LinkAssociation<NorthT, EastT, SouthT, WestT> {
    fn construct(north: NorthT, east: EastT, south: SouthT, west: WestT) -> Self;
    fn north(&self) -> NorthT;
    fn east(&self) -> EastT;
    fn south(&self) -> SouthT;
    fn west(&self) -> WestT;
}

impl<NorthT, EastT, SouthT, WestT> LinkAssociation<NorthT, EastT, SouthT, WestT>
    for (NorthT, EastT, SouthT, WestT)
where
    NorthT: Clone,
    EastT: Clone,
    SouthT: Clone,
    WestT: Clone,
{
    fn construct(north: NorthT, east: EastT, south: SouthT, west: WestT) -> Self {
        (north, east, south, west)
    }

    fn north(&self) -> NorthT {
        self.0.clone()
    }

    fn east(&self) -> EastT {
        self.1.clone()
    }

    fn south(&self) -> SouthT {
        self.2.clone()
    }

    fn west(&self) -> WestT {
        self.3.clone()
    }
}

impl<NorthT, EastT, SouthT, WestT> LinkAssociation<NorthT, EastT, SouthT, WestT>
    for ((NorthT, EastT), (SouthT, WestT))
where
    NorthT: Clone,
    EastT: Clone,
    SouthT: Clone,
    WestT: Clone,
{
    fn construct(north: NorthT, east: EastT, south: SouthT, west: WestT) -> Self {
        ((north, east), (south, west))
    }

    fn north(&self) -> NorthT {
        self.0 .0.clone()
    }

    fn east(&self) -> EastT {
        self.0 .1.clone()
    }

    fn south(&self) -> SouthT {
        self.1 .0.clone()
    }

    fn west(&self) -> WestT {
        self.1 .1.clone()
    }
}

impl<NorthT, EastT, SouthT, WestT> LinkAssociation<NorthT, EastT, SouthT, WestT>
    for (((NorthT, EastT), SouthT), WestT)
where
    NorthT: Clone,
    EastT: Clone,
    SouthT: Clone,
    WestT: Clone,
{
    fn construct(north: NorthT, east: EastT, south: SouthT, west: WestT) -> Self {
        (((north, east), south), west)
    }

    fn north(&self) -> NorthT {
        self.0 .0 .0.clone()
    }

    fn east(&self) -> EastT {
        self.0 .0 .1.clone()
    }

    fn south(&self) -> SouthT {
        self.0 .1.clone()
    }

    fn west(&self) -> WestT {
        self.1.clone()
    }
}
