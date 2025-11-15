# Containerfile for Claude Code Router agent layer
# This layer installs Claude Code and Claude Code Router for AI-assisted coding
# Claude Code Router wraps Claude Code and routes requests to different models

ARG BASE_IMAGE
FROM ${BASE_IMAGE}

USER root

# Install Claude Code (required by Claude Code Router) and Claude Code Router
RUN npm install -g @anthropic-ai/claude-code @musistudio/claude-code-router

USER agent
WORKDIR /workspace
