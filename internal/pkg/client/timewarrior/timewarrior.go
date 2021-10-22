package timewarrior

import (
	"context"
	"encoding/json"
	"fmt"
	"os/exec"
	"regexp"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	dateFormat      string = "2006-01-02T15:04:05"
	ParseDateFormat string = "20060102T150405Z"
)

// FetchEntry represents the entry exported from Timewarrior.
type FetchEntry struct {
	ID         int      `json:"id"`
	Start      string   `json:"start"`
	End        string   `json:"end"`
	Tags       []string `json:"tags"`
	Annotation string   `json:"annotation"`
}

// ClientOpts is the client specific options, extending client.BaseClientOpts.
// Since Timewarrior is a CLI tool, hence it has no API we could call on HTTP.
// Although client.HTTPClientOpts is part of client.BaseClientOpts, we are
// not using that as part of this integration, instead we are defining the path
// of the executable (Command) and the command arguments used for export
// (CommandArguments).
type ClientOpts struct {
	client.BaseClientOpts
	Command            string
	CommandArguments   []string
	CommandCtxExecutor func(ctx context.Context, name string, arg ...string) *exec.Cmd
	UnbillableTag      string
	ClientTagRegex     string
	ProjectTagRegex    string
}

type timewarriorClient struct {
	opts             *ClientOpts
	clientTagRegex   *regexp.Regexp
	projectTagRegex  *regexp.Regexp
	tagsAsTasksRegex *regexp.Regexp
}

func (c *timewarriorClient) executeCommand(ctx context.Context, subcommand string, entries *[]FetchEntry, opts *client.FetchOpts) error {
	arguments := []string{subcommand}

	arguments = append(
		arguments,
		[]string{
			"from", opts.Start.Format(dateFormat),
			"to", opts.End.Format(dateFormat),
		}...,
	)

	arguments = append(arguments, c.opts.CommandArguments...)

	out, err := c.opts.CommandCtxExecutor(ctx, c.opts.Command, arguments...).Output() // #nosec G204
	if err != nil {
		return err
	}

	if err = json.Unmarshal(out, &entries); err != nil {
		return err
	}

	return nil
}

func (c *timewarriorClient) parseEntry(entry FetchEntry) (worklog.Entries, error) {
	var entries worklog.Entries

	startDate, err := time.ParseInLocation(ParseDateFormat, entry.Start, time.Local)
	if err != nil {
		return nil, err
	}

	endDate, err := time.ParseInLocation(ParseDateFormat, entry.End, time.Local)
	if err != nil {
		return nil, err
	}

	worklogEntry := worklog.Entry{
		Summary:            entry.Annotation,
		Notes:              entry.Annotation,
		Start:              startDate,
		BillableDuration:   endDate.Sub(startDate),
		UnbillableDuration: 0,
	}

	for _, tag := range entry.Tags {
		if tag == c.opts.UnbillableTag {
			worklogEntry.UnbillableDuration = worklogEntry.BillableDuration
			worklogEntry.BillableDuration = 0
		} else if c.opts.ClientTagRegex != "" && c.clientTagRegex.MatchString(tag) {
			worklogEntry.Client = worklog.IDNameField{
				ID:   tag,
				Name: tag,
			}
		} else if c.opts.ProjectTagRegex != "" && c.projectTagRegex.MatchString(tag) {
			worklogEntry.Project = worklog.IDNameField{
				ID:   tag,
				Name: tag,
			}
		} else if c.opts.TagsAsTasksRegex != "" && c.tagsAsTasksRegex.MatchString(tag) {
			worklogEntry.Task = worklog.IDNameField{
				ID:   tag,
				Name: tag,
			}
		}
	}

	// If the task was not found in tags, make sure to set it to annotation
	if !worklogEntry.Task.IsComplete() {
		worklogEntry.Task = worklog.IDNameField{
			ID:   entry.Annotation,
			Name: entry.Annotation,
		}
	}

	if c.opts.TagsAsTasks && len(entry.Tags) > 0 {
		var tags []worklog.IDNameField
		for _, tag := range entry.Tags {
			tags = append(tags, worklog.IDNameField{
				ID:   tag,
				Name: tag,
			})
		}

		splitEntries := worklogEntry.SplitByTagsAsTasks(worklogEntry.Summary, c.tagsAsTasksRegex, tags)
		entries = append(entries, splitEntries...)
	} else {
		entries = append(entries, worklogEntry)
	}

	return entries, nil
}

func (c *timewarriorClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) (worklog.Entries, error) {
	var fetchedEntries []FetchEntry
	if err := c.executeCommand(ctx, "export", &fetchedEntries, opts); err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var entries worklog.Entries
	for _, entry := range fetchedEntries {
		parsedEntries, err := c.parseEntry(entry)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		entries = append(entries, parsedEntries...)
	}

	return entries, nil
}

// NewClient returns a new Timewarrior client.
func NewClient(opts *ClientOpts) (client.Fetcher, error) {
	clientTagRegex, err := regexp.Compile(opts.ClientTagRegex)
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	projectTagRegex, err := regexp.Compile(opts.ProjectTagRegex)
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	tagsAsTasksRegex, err := regexp.Compile(opts.TagsAsTasksRegex)
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	return &timewarriorClient{
		opts:             opts,
		clientTagRegex:   clientTagRegex,
		projectTagRegex:  projectTagRegex,
		tagsAsTasksRegex: tagsAsTasksRegex,
	}, nil
}
