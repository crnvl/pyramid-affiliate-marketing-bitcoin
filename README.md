# c3pixelflut
This is a lowkey overengineered [pixelflut](https://c3pixelflut.de) client, without a name yet .-.

## Filter system
Filters are applied on every frame. Filters are always applied in the same order and always on the base image, so to propagate changes between frames they have to be saved inside of the filter, which is mutably accessible.
To implement a filter something has to implement the `filter::Filter` trait.
The `transform_buffer` function takes ultiple arguments:

- `buffer: &mut Vec<crate::Pixel>`
  - This is the buffer the filter is applied on.
- `restore: Option<&mut Vec<crate::Pixel>>`
  - The restore buffer is a bit more complicated. It is only set, if the restore mode is enabled (`-r`) and is relevant for filters, that move pixels arouns, as the restore mode lets the renderer restore pixels, that have been occupied but aren't occupied anymore. Filters that move pixels have to predict, which pixels are going to not be occupied anymore in the next frame. This prediction can be inprecise and better includes more pixels than needed, than less. But the more pixel it includes the more needless overhead is produced every frame, slowing down the whole efficiency. The colors of the Pixels are not relevant, as the renderer fetches these from the server before rendering a frame.
