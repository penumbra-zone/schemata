use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Colon, Comma, Paren, Semi},
    Attribute, Ident, Result, Type,
};

#[derive(Clone, Debug, Default)]
pub struct Input {
    pub attrs: Vec<Attribute>,
    pub children: Vec<Child>,
}

#[derive(Clone, Debug)]
pub enum Child {
    Leaf {
        segment: Segment,
        colon_token: Colon,
        ty: Box<Type>,
        semi_token: Semi,
    },
    Internal {
        segment: Segment,
        brace_token: Brace,
        children: Vec<Child>,
    },
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    pub params: Option<Parameters>,
}

#[derive(Clone, Debug)]
pub struct Parameters {
    pub paren_token: Paren,
    pub params: Punctuated<Parameter, Comma>,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub attrs: Vec<Attribute>,
    pub name: Box<Ident>,
    pub colon_token: Colon,
    pub ty: Box<Type>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_inner)?;

        let mut children = Vec::new();
        while !input.is_empty() {
            children.push(input.parse()?);
        }

        Ok(Input { attrs, children })
    }
}

impl Parse for Child {
    fn parse(input: ParseStream) -> Result<Self> {
        // Common to both leaves and internal nodes:
        let mut segment = input.parse()?;

        // Look ahead to determine what kind of node we are
        let lookahead = input.lookahead1();

        // If there's a colon, that means we're a leaf, so parse the colon and a type
        Ok(if lookahead.peek(Colon) {
            Child::Leaf {
                segment,
                colon_token: input.parse()?,
                ty: input.parse()?,
                semi_token: input.parse()?,
            }

        // Otherwise, if there's a brace, that means we're an internal node, so parse that
        } else if lookahead.peek(Brace) {
            let content;
            let brace_token = braced!(content in input);

            // If there are outer attributes inside this item, attach them to the inner
            // attributes we parsed above
            segment.attrs.extend(content.call(Attribute::parse_inner)?);

            // Parse all the children
            let mut children = Vec::new();
            while !content.is_empty() {
                children.push(content.parse()?);
            }

            Child::Internal {
                segment,
                brace_token,
                children,
            }

        // If neither of those hold, this is a parse error
        } else {
            return Err(lookahead.error());
        })
    }
}

impl Parse for Segment {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Segment {
            attrs: input.call(Attribute::parse_outer)?,
            name: input.parse()?,
            params: {
                let lookahead = input.lookahead1();
                if lookahead.peek(Paren) {
                    Some(input.parse()?)
                } else {
                    None
                }
            },
        })
    }
}

impl Parse for Parameters {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Parameters {
            paren_token: parenthesized!(content in input),
            params: Punctuated::parse_terminated(&content)?,
        })
    }
}

impl Parse for Parameter {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Parameter {
            attrs: input.call(Attribute::parse_outer)?,
            name: input.parse()?,
            colon_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}
