// Base parsing methods for FullXbrlParser

impl<'a> FullXbrlParser<'a> {
    #[inline(always)]
    fn read_tag_name(&mut self) -> Result<&'a str> {
        let start = self.scanner.pos;
        while let Some(ch) = self.scanner.peek() {
            if ch == b' ' || ch == b'>' || ch == b'/' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                break;
            }
            self.scanner.advance(1);
        }
        let end = self.scanner.pos;
        
        if start == end {
            return Err(Error::Parse("Empty tag name".to_string()));
        }
        
        std::str::from_utf8(&self.scanner.data[start..end])
            .map_err(|_| Error::Parse("Invalid UTF-8 in tag name".to_string()))
    }

    #[inline(always)]
    fn parse_attributes(&mut self) -> Result<Vec<(&'a str, &'a str)>> {
        let mut attrs = Vec::new();
        
        loop {
            self.scanner.skip_whitespace();
            
            match self.scanner.peek() {
                Some(b'>') => {
                    // End of tag
                    break;
                }
                Some(b'/') => {
                    // Self-closing tag
                    self.scanner.advance(1);
                    if self.scanner.peek() == Some(b'>') {
                        break;
                    }
                }
                None => return Err(Error::Parse("Unexpected EOF in attributes".to_string())),
                _ => {}
            }
            
            let name_start = self.scanner.pos;
            while let Some(ch) = self.scanner.peek() {
                if ch == b'=' || ch == b' ' || ch == b'>' || ch == b'/' {
                    break;
                }
                self.scanner.advance(1);
            }
            
            if self.scanner.pos == name_start {
                break; // No more attributes
            }
            
            let name = std::str::from_utf8(&self.scanner.data[name_start..self.scanner.pos])
                .map_err(|_| Error::Parse("Invalid UTF-8 in attribute name".to_string()))?;
            
            self.scanner.skip_whitespace();
            
            if self.scanner.peek() != Some(b'=') {
                continue;
            }
            self.scanner.advance(1);
            
            self.scanner.skip_whitespace();
            
            let quote = self.scanner.peek()
                .ok_or_else(|| Error::Parse("Expected quote".to_string()))?;
            
            if quote != b'"' && quote != b'\'' {
                return Err(Error::Parse("Expected quote in attribute".to_string()));
            }
            
            self.scanner.advance(1);
            let value_start = self.scanner.pos;
            
            while let Some(ch) = self.scanner.peek() {
                if ch == quote {
                    break;
                }
                self.scanner.advance(1);
            }
            
            let value = std::str::from_utf8(&self.scanner.data[value_start..self.scanner.pos])
                .map_err(|_| Error::Parse("Invalid UTF-8 in attribute value".to_string()))?;
            
            self.scanner.advance(1); // Skip closing quote
            
            attrs.push((name, value));
        }
        
        Ok(attrs)
    }

    #[inline(always)]
    fn skip_to_tag_end(&mut self) -> Result<()> {
        while let Some(ch) = self.scanner.peek() {
            if ch == b'>' {
                self.scanner.advance(1);
                return Ok(());
            }
            self.scanner.advance(1);
        }
        Err(Error::Parse("Expected '>'".to_string()))
    }

    #[inline(always)]
    fn read_text_content(&mut self) -> Result<&'a str> {
        let start = self.scanner.pos;
        while let Some(ch) = self.scanner.peek() {
            if ch == b'<' {
                break;
            }
            self.scanner.advance(1);
        }
        
        let text = std::str::from_utf8(&self.scanner.data[start..self.scanner.pos])
            .map_err(|_| Error::Parse("Invalid UTF-8 in text content".to_string()))?;
        
        Ok(text.trim())
    }

    #[inline(always)]
    fn skip_element_from_tag(&mut self) -> Result<()> {
        // We've already read the tag name, now skip to end of opening tag
        self.skip_to_tag_end()?;
        
        // Check if it was self-closing
        if self.scanner.pos >= 2 && self.scanner.data[self.scanner.pos - 2] == b'/' {
            return Ok(()); // Self-closing tag, we're done
        }
        
        // Skip element content and find matching closing tag
        let mut depth = 1;
        
        while depth > 0 && !self.scanner.is_eof() {
            // Find next tag
            while let Some(ch) = self.scanner.peek() {
                if ch == b'<' {
                    break;
                }
                self.scanner.advance(1);
            }
            
            if self.scanner.is_eof() {
                break;
            }
            
            self.scanner.advance(1); // consume '<'
            
            if self.scanner.peek() == Some(b'/') {
                depth -= 1;
            } else if self.scanner.peek() != Some(b'!') && self.scanner.peek() != Some(b'?') {
                // Check if it's a self-closing tag
                let mut is_self_closing = false;
                let _saved_pos = self.scanner.pos;
                
                // Skip to end of tag to check
                while let Some(ch) = self.scanner.peek() {
                    if ch == b'/' {
                        if self.scanner.pos + 1 < self.scanner.data.len() 
                            && self.scanner.data[self.scanner.pos + 1] == b'>' {
                            is_self_closing = true;
                        }
                    }
                    if ch == b'>' {
                        self.scanner.advance(1);
                        break;
                    }
                    self.scanner.advance(1);
                }
                
                if !is_self_closing {
                    depth += 1;
                }
                
                continue;
            }
            
            // Skip to end of this tag
            while let Some(ch) = self.scanner.peek() {
                if ch == b'>' {
                    self.scanner.advance(1);
                    break;
                }
                self.scanner.advance(1);
            }
        }
        
        Ok(())
    }

    #[inline(always)]
    fn skip_processing_instruction(&mut self) -> Result<()> {
        // Skip until ?>
        while !self.scanner.is_eof() {
            if self.scanner.peek() == Some(b'?') {
                self.scanner.advance(1);
                if self.scanner.peek() == Some(b'>') {
                    self.scanner.advance(1);
                    return Ok(());
                }
            } else {
                self.scanner.advance(1);
            }
        }
        Err(Error::Parse("Unclosed processing instruction".to_string()))
    }

    #[inline(always)]
    fn skip_comment(&mut self) -> Result<()> {
        // Skip until -->
        while !self.scanner.is_eof() {
            if self.scanner.peek() == Some(b'-') {
                self.scanner.advance(1);
                if self.scanner.peek() == Some(b'-') {
                    self.scanner.advance(1);
                    if self.scanner.peek() == Some(b'>') {
                        self.scanner.advance(1);
                        return Ok(());
                    }
                }
            } else {
                self.scanner.advance(1);
            }
        }
        Err(Error::Parse("Unclosed comment".to_string()))
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
