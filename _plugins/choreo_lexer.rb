# _plugins/choreo_lexer.rb

require 'rouge'

class Choreo < Rouge::RegexLexer
  title "Choreo"
  desc "A DSL for behavior-driven testing (github.com/cladam/choreo)"
  tag 'choreo'
  filenames '*.chor'
  mimetypes 'text/x-choreo'

  # --- Word Lists ---
  KEYWORD_DECLARATION = %w(
    feature actors settings background scenario after test var
  ).freeze

  KEYWORD_STEP = %w(
    given when then
  ).freeze

  BUILTIN_LITERAL = %w(
    Web Terminal FileSystem true false
  ).freeze

  COMMANDS_AND_ASSERTIONS = %w(
    wait set_header set_cookie http_get http_post clear_header clear_cookie
    run type wait_for_text create_file delete_file append_to_file
    response_status_is response_time_is_below response_header_is response_body_is
    response_body_contains expect_exit_code stdout_contains stderr_contains
    stdout_not_contains stderr_not_contains file_exists file_not_exists
    file_contains file_not_contains
  ).freeze

  # --- Lexer States ---
  state :root do
    rule %r/\s+/, Text::Whitespace
    rule %r/#.*$/, Comment::Single

    # Keywords that are followed by other text on the same line
    rule %r/^(feature|scenario|test|var)\b/, Keyword::Declaration, :title_line

    # Standalone keywords
    rule %r/\b(#{KEYWORD_DECLARATION.join('|')})\b/, Keyword::Declaration
    rule %r/\b(#{KEYWORD_STEP.join('|')})\b:/, Keyword # Look for the colon
    rule %r/\b(#{BUILTIN_LITERAL.join('|')})\b/, Name::Builtin
    rule %r/\b(#{COMMANDS_AND_ASSERTIONS.join('|')})\b/, Name::Function

    # Punctuation and operators
    rule %r/[{}=:]|>=/, Punctuation

    # Strings in double quotes
    rule %r/"/, Str::Double, :string

    # Numbers and time values
    rule %r/\b\d+s?\b/, Num
  end

  # This state handles the rest of a line after a keyword like 'feature'
  state :title_line do
    rule %r/"/, Str::Double, :string  # Capture strings in the title
    rule %r/\n/, Text::Whitespace, :pop! # Exit state on newline
    rule %r/[^"\n]+/, Text # Everything else is plain text
  end

  # This state handles content inside strings
  state :string do
    rule %r/"/, Str::Double, :pop!
    rule %r/\$\{.*?}/, Name::Variable # Interpolation
    rule %r/[^"]+/, Str::Double
  end
end
