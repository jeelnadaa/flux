//! Container metadata, tags, gapless playback information, and cover art.

/// Encoder delay, padding, and original sample count for gapless playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GaplessInfo {
    /// Encoder delay / priming samples at the start of the audio stream.
    pub encoder_delay: u32,
    /// Encoder padding samples at the end of the audio stream.
    pub encoder_padding: u32,
    /// Exact original audio sample count, if specified by encoder header.
    pub original_samples: Option<u64>,
}

/// Embedded picture / album artwork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverArt {
    /// MIME type of the picture (e.g. "image/jpeg", "image/png").
    pub mime_type: String,
    /// Picture role / type (e.g. "CoverFront", "CoverBack", "Artist").
    pub picture_type: String,
    /// Optional picture description.
    pub description: Option<String>,
    /// Raw image bytes.
    pub data: Vec<u8>,
}

/// Rich container metadata tags extracted from audio streams.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContainerMetadata {
    /// Track title.
    pub title: Option<String>,
    /// Artist / Performer.
    pub artist: Option<String>,
    /// Album title.
    pub album: Option<String>,
    /// Album artist.
    pub album_artist: Option<String>,
    /// Composer.
    pub composer: Option<String>,
    /// Track date / release year.
    pub date: Option<String>,
    /// Musical genre.
    pub genre: Option<String>,
    /// Track number in album.
    pub track_number: Option<u32>,
    /// Total number of tracks in album.
    pub track_total: Option<u32>,
    /// Disc / media number.
    pub disc_number: Option<u32>,
    /// Total number of discs.
    pub disc_total: Option<u32>,
    /// Free-form comment or annotation.
    pub comment: Option<String>,
    /// Copyright notice.
    pub copyright: Option<String>,
    /// Software / encoder used.
    pub software: Option<String>,
    /// Gapless priming and padding information.
    pub gapless_info: Option<GaplessInfo>,
    /// Primary embedded cover artwork.
    pub cover_art: Option<CoverArt>,
}

impl ContainerMetadata {
    /// Merges another `ContainerMetadata` into this one, keeping existing values.
    pub fn merge(&mut self, other: ContainerMetadata) {
        if self.title.is_none() {
            self.title = other.title;
        }
        if self.artist.is_none() {
            self.artist = other.artist;
        }
        if self.album.is_none() {
            self.album = other.album;
        }
        if self.album_artist.is_none() {
            self.album_artist = other.album_artist;
        }
        if self.composer.is_none() {
            self.composer = other.composer;
        }
        if self.date.is_none() {
            self.date = other.date;
        }
        if self.genre.is_none() {
            self.genre = other.genre;
        }
        if self.track_number.is_none() {
            self.track_number = other.track_number;
        }
        if self.track_total.is_none() {
            self.track_total = other.track_total;
        }
        if self.disc_number.is_none() {
            self.disc_number = other.disc_number;
        }
        if self.disc_total.is_none() {
            self.disc_total = other.disc_total;
        }
        if self.comment.is_none() {
            self.comment = other.comment;
        }
        if self.copyright.is_none() {
            self.copyright = other.copyright;
        }
        if self.software.is_none() {
            self.software = other.software;
        }
        if self.gapless_info.is_none() {
            self.gapless_info = other.gapless_info;
        }
        if self.cover_art.is_none() {
            self.cover_art = other.cover_art;
        }
    }
}
