# _plugins/choreo_lexer.rb

require 'rouge'

class Choreo < Rouge::RegexLexer
  title "Choreo"
  desc "A DSL for behavior-driven testing (github.com/cladam/choreo)"
  tag 'choreo'
  filenames '*.chor'
  mimetypes 'text/x-choreo'

  state :root do
    # Whitespace and Comments
    rule %r/\s+/m, Text::Whitespace
    rule %r/#.*$/, Comment::Single

    # Keywords that start a block or declaration
    rule %r/^(feature|actors|settings|background|scenario|after|var|test)\b/, Keyword::Declaration

    # Step keywords with a colon
    rule %r/^\s*(given|when|then):/m, Keyword

    # Built-in actors and literal values
    rule %r/\b(Web|Terminal|FileSystem|true|false)\b/, Name::Builtin
    
    # All commands and assertions
    rule %r/\b(wait|set_header|set_cookie|http_get|http_post|clear_header|clear_cookie|run|type|wait_for_text|create_file|delete_file|append_to_file|response_status_is|response_time_is_below|response_header_is|response_body_is|response_body_contains|expect_exit_code|stdout_contains|stderr_contains|stdout_not_contains|stderr_not_contains|file_exists|file_not_exists|file_contains|file_not_contains)\b/, Name::Function

    # Punctuation and operators
    rule %r/[{}=:]|>=/, Punctuation

    # Strings, which can contain variables
    rule %r/"/, Str::Double, :string

    # Numbers and time values
    rule %r/\b\d+s?\b/, Num
    
    # Any other text (variable names, test names etc.)
    rule %r/[a-zA-Z_][a-zA-Z0-9_]*/, Text
  end

  # State for handling content inside strings
  state :string do
    rule %r/"/, Str::Double, :pop!
    rule %r/\$\{.*?}/, Name::Variable
    rule %r/[^"]+/, Str::Double
  end
end
