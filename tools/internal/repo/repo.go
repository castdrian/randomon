package repo

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
)

const PinnedUpstreamCommit = "7f00eb866ed35c8fe3963f078b6a2e0979dc2b8c"

func Root() string {
	if override := os.Getenv("RANDOMON_ROOT"); override != "" {
		return override
	}
	_, source, _, _ := runtime.Caller(0)
	return filepath.Clean(filepath.Join(filepath.Dir(source), "..", "..", ".."))
}

func Path(parts ...string) string {
	return filepath.Join(append([]string{Root()}, parts...)...)
}

func Upstream() string {
	if override := os.Getenv("RANDOMON_UPSTREAM"); override != "" {
		return override
	}
	return Path("target", "upstream")
}

func UpstreamSource() string {
	return filepath.Join(Upstream(), "src")
}

func Run(directory string, name string, arguments ...string) error {
	command := exec.Command(name, arguments...)
	command.Dir = directory
	command.Stdout = os.Stdout
	command.Stderr = os.Stderr
	if err := command.Run(); err != nil {
		return fmt.Errorf("%s failed: %w", name, err)
	}
	return nil
}

func RunWithEnv(directory string, environment []string, name string, arguments ...string) error {
	command := exec.Command(name, arguments...)
	command.Dir = directory
	command.Env = append(os.Environ(), environment...)
	command.Stdout = os.Stdout
	command.Stderr = os.Stderr
	if err := command.Run(); err != nil {
		return fmt.Errorf("%s failed: %w", name, err)
	}
	return nil
}

func JavaSources(directories ...string) ([]string, error) {
	var sources []string
	for _, directory := range directories {
		err := filepath.WalkDir(directory, func(path string, entry os.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if !entry.IsDir() && filepath.Ext(path) == ".java" {
				sources = append(sources, path)
			}
			return nil
		})
		if err != nil {
			return nil, err
		}
	}
	return sources, nil
}

func Fail(err error) {
	fmt.Fprintln(os.Stderr, err)
	os.Exit(1)
}

func ClassPath(entries ...string) string {
	return filepath.Join(entries[0]) + joinRest(entries[1:])
}

func joinRest(entries []string) string {
	joined := ""
	for _, entry := range entries {
		joined += string(os.PathListSeparator) + entry
	}
	return joined
}
