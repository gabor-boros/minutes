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
	// DefaultPageSizeParam used by paginated fetchers setting page size parameter.
	DefaultPageSizeParam string = "per_page"
	// DefaultPageParam used by paginated fetchers setting the page parameter.
	DefaultPageParam string = "page"
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

type PaginatedFetchResponse struct {
	EntriesPerPage int
	TotalEntries   int
}

type PaginatedFetchFunc = func(context.Context, string) (interface{}, *PaginatedFetchResponse, error)
type PaginatedParseFunc = func(interface{}) (worklog.Entries, error)

type PaginatedFetchOpts struct {
	URL           string
	PageSize      int
	PageSizeParam string
	PageParam     string

	FetchFunc PaginatedFetchFunc
	ParseFunc PaginatedParseFunc
}
