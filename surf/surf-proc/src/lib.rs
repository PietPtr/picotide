use proc_macro2::{Group, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Ident, Token,
};

struct NameDef {
    _name_token: Ident,
    _eq: Token![=],
    name: Ident,
    _semi: Token![;],
    rest: TokenStream2,
}

fn parse_hard_string(input: ParseStream, string: &str, error_message: &str) -> syn::Result<Ident> {
    let identifier = input.parse::<Ident>()?;

    if !identifier.to_string().eq(string) {
        return Err(syn::Error::new(identifier.span(), error_message));
    }

    Ok(identifier)
}

impl Parse for NameDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(NameDef {
            _name_token: parse_hard_string(
                input,
                "Name",
                concat!(
                    "First define the name of the statemachine, e.g.:\n",
                    "    Name = MyStateMachine;"
                ),
            )?,
            _eq: input.parse()?,
            name: input.parse()?,
            _semi: input.parse()?,
            rest: input.parse()?,
        })
    }
}

#[derive(Debug)]
enum StructOrEnum {
    Struct,
    Enum,
}

impl Parse for StructOrEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![struct]) {
            input.parse::<Token![struct]>()?;
            Ok(StructOrEnum::Struct)
        } else if lookahead.peek(Token![enum]) {
            input.parse::<Token![enum]>()?;
            Ok(StructOrEnum::Enum)
        } else {
            Err(lookahead.error())
        }
    }
}

struct ConfigurationDef {
    /// struct or enum
    kind: StructOrEnum,
    /// literally the word Configuration
    _configuration_token: Ident,
    /// Body of the type
    group: Group,
    rest: TokenStream2,
}

impl Parse for ConfigurationDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ConfigurationDef {
            kind: input.parse()?,
            _configuration_token: parse_hard_string(
                input,
                "Configuration",
                concat!(
                    "Define the configuration of the statemachine exactly like:\n",
                    "    struct Configuration = { ... }\n",
                    "    enum Configuration = { ... }\n"
                ),
            )?,
            group: input.parse()?,
            rest: input.parse()?,
        })
    }
}

struct StateDef {
    _enum: Token![enum],
    _state_token: Ident,
    group: Group,
    rest: TokenStream2,
}

impl Parse for StateDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![struct]) {
            return Err(syn::Error::new(
                input.span(),
                "State cannot be a struct and must be an enum.",
            ));
        }

        Ok(StateDef {
            _enum: input.parse()?,
            _state_token: parse_hard_string(
                input,
                "State",
                concat!(
                    "Define the state enum of the state machine exactly like:\n",
                    "    enum State { ... }"
                ),
            )?,
            group: input.parse()?,
            rest: input.parse()?,
        })
    }
}

struct InputDef {
    kind: StructOrEnum,
    input_token: Ident,
    group: Group,
    rest: TokenStream2,
}

impl Parse for InputDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(InputDef {
            kind: input.parse()?,
            input_token: parse_hard_string(
                input,
                "Input",
                concat!(
                    "Define the input type of the state machine exactly like:\n",
                    "    struct Input = { ... }\n",
                    "    enum Input = { ... }"
                ), // TODO: type alias?
            )?,
            group: input.parse()?,
            rest: input.parse()?,
        })
    }
}

struct OutputDef {
    kind: StructOrEnum,
    input_token: Ident,
    group: Group,
    rest: TokenStream2,
}

impl Parse for OutputDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(OutputDef {
            kind: input.parse()?,
            input_token: parse_hard_string(
                input,
                "Output",
                concat!(
                    "Define the output type of the state machine exactly like:\n",
                    "    struct Output = { ... }\n",
                    "    enum Output = { ... }"
                ), // TODO: type alias?
            )?,
            group: input.parse()?,
            rest: input.parse()?,
        })
    }
}

struct ImplDef {
    _impl: Token![impl],
    group: Group,
}

impl Parse for ImplDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ImplDef {
            _impl: input.parse()?,
            group: input.parse()?,
        })
    }
}

/// Parses a given state machine definition, then reworks it into a nice compilable trait implementation of StateMachine
#[proc_macro]
pub fn state_machine(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let name_definition = syn::parse_macro_input!(input as NameDef);

    let rest = name_definition.rest.into();
    let configuration_struct = syn::parse_macro_input!(rest as ConfigurationDef);

    let rest = configuration_struct.rest.into();
    let state_struct = syn::parse_macro_input!(rest as StateDef);

    let rest = state_struct.rest.into();
    let input_struct = syn::parse_macro_input!(rest as InputDef);

    let rest = input_struct.rest.into();
    let output_struct = syn::parse_macro_input!(rest as OutputDef);

    let rest = output_struct.rest.into();
    let impl_struct = syn::parse_macro_input!(rest as ImplDef);

    let expanded = quote! {
        const HOI: &str = "yea";
    };

    proc_macro::TokenStream::from(expanded)
}
