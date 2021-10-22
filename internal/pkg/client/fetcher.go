package client

import (
	"context"
	"errors"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	// DefaultPageSize used by paginated fetchers setting the fetched page size.
	// The minimum page sizes can be different per client, but the 50 items per
	// page is usually supported everywhere.
	DefaultPageSize int = 50
	// DefaultMaxPageSize used by paginated fetchers setting the maximum entries
	// per page. The maximum page sizes can be different per client, but the
	// 250 items per page is usually supported everywhere.
	DefaultMaxPageSize int = 250
)

var (
	// ErrFetchEntries wraps the error when fetch failed.
	ErrFetchEntries = errors.New("failed to fetch entries")
)

// FetchOpts specifies the only options for Fetchers.
// In contract to the BaseClientOpts, these options shall not be extended or
// overridden.
type FetchOpts struct {
	User  string
	Start time.Time
	End   time.Time
}

// Fetcher specifies the functions used to fetch worklog entries.
type Fetcher interface {
	// FetchEntries from a given source and return the list of worklog entries
	// If the fetching resulted in an error, the list of worklog entries will be
	// nil and an error will return.
	FetchEntries(ctx context.Context, opts *FetchOpts) (worklog.Entries, error)
}
