package main

import "github.com/gabor-boros/minutes/cmd/root"

var (
	version string
	commit  string
	date    string
)

func main() {
	root.Execute(version, commit, date)
}
