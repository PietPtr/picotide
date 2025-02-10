use proc_macro2::{Group, Punct, Spacing, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    visit_mut::VisitMut,
    ExprGroup, Ident, Token,
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

impl StructOrEnum {
    fn token(&self) -> TokenStream2 {
        match self {
            StructOrEnum::Struct => "struct".parse::<TokenStream2>().unwrap(),
            StructOrEnum::Enum => "enum".parse::<TokenStream2>().unwrap(),
        }
    }
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
    configuration_token: Ident,
    /// Body of the type
    group: Group,
    rest: TokenStream2,
}

impl Parse for ConfigurationDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ConfigurationDef {
            kind: input.parse()?,
            configuration_token: parse_hard_string(
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
    state_token: Ident,
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
            state_token: parse_hard_string(
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

impl ImplDef {
    pub fn validate_and_convert(
        &mut self,
        state_enum_name: Ident,
        input_type_name: Ident,
        output_type_name: Ident,
    ) {
        // Iterate through the top level tokens, among these will be identifiers which are
        // the state names and those'll have to be prefixed by the state type.
        // All encountered groups should have replace_ident_in_group applied to them to
        // replace the state, input, and output type.
        let new_stream = self
            .group
            .stream()
            .into_iter()
            .flat_map(|token| match token {
                TokenTree::Group(group) => {
                    let new_group =
                        replace_ident_in_group(&group, "State", &state_enum_name.to_string());
                    let new_group =
                        replace_ident_in_group(&new_group, "Input", &input_type_name.to_string());
                    let new_group =
                        replace_ident_in_group(&new_group, "Output", &output_type_name.to_string());

                    vec![TokenTree::Group(new_group)].into_iter()
                }
                TokenTree::Ident(ident) => vec![
                    TokenTree::Ident(state_enum_name.clone()),
                    TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                    TokenTree::Punct(Punct::new(':', Spacing::Alone)),
                    TokenTree::Ident(ident),
                ]
                .into_iter(),
                token => vec![token].into_iter(), // Wrap other tokens in a single-item iterator
            })
            .collect();

        self.group = Group::new(self.group.delimiter(), new_stream);

        // panic!("{:#?}", self.group);
    }
}

fn replace_ident_in_group(group: &Group, target: &str, replacement: &str) -> Group {
    let new_stream = group
        .stream()
        .into_iter()
        .map(|token| match token {
            TokenTree::Ident(ident) if ident == target => {
                TokenTree::Ident(Ident::new(replacement, ident.span()))
            }
            TokenTree::Group(inner_group) => TokenTree::Group(Group::new(
                inner_group.delimiter(),
                replace_ident_in_group(&inner_group, target, replacement).stream(),
            )),
            other => other,
        })
        .collect();

    Group::new(group.delimiter(), new_stream)
}

/// Parses a given state machine definition, then reworks it into a nice compilable trait implementation of StateMachine
#[proc_macro]
pub fn state_machine(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse definition
    let name_def = syn::parse_macro_input!(input as NameDef);

    let rest = name_def.rest.into();
    let configuration_def = syn::parse_macro_input!(rest as ConfigurationDef);

    let rest = configuration_def.rest.into();
    let state_def = syn::parse_macro_input!(rest as StateDef);

    let rest = state_def.rest.into();
    let input_def = syn::parse_macro_input!(rest as InputDef);

    let rest = input_def.rest.into();
    let output_def = syn::parse_macro_input!(rest as OutputDef);

    let rest = output_def.rest.into();
    let mut impl_def = syn::parse_macro_input!(rest as ImplDef);

    // Construct valid Rust
    let state_machine_name = name_def.name.clone();

    let configuration_type_name = Ident::new(
        &format!("{}Configuration", name_def.name),
        configuration_def.configuration_token.span(),
    );
    let configuration_kind = configuration_def.kind.token();
    let configuration_type_body = configuration_def.group;

    let state_enum_name = Ident::new(
        &format!("{}State", name_def.name),
        state_def.state_token.span(),
    );
    let state_enum_body = state_def.group;

    let input_kind = input_def.kind.token();
    let input_type_name = Ident::new(
        &format!("{}Input", name_def.name),
        input_def.input_token.span(),
    );
    let input_body = input_def.group;

    let output_kind = output_def.kind.token();
    let output_type_name = Ident::new(
        &format!("{}Output", name_def.name),
        output_def.input_token.span(),
    );
    let output_body = output_def.group;

    impl_def.validate_and_convert(
        state_enum_name.clone(),
        input_type_name.clone(),
        output_type_name.clone(),
    );
    let impl_body = impl_def.group;

    // TODO: make all the fields public in the configuration, input, and output?

    let expanded = quote! {
        pub struct #state_machine_name <'a> {
            state: #state_enum_name,
            configuration: &'a #configuration_type_name
        }

        pub #configuration_kind #configuration_type_name #configuration_type_body
        pub enum #state_enum_name #state_enum_body
        pub #input_kind #input_type_name #input_body
        pub #output_kind #output_type_name #output_body

        impl<'a> surf_lang::StateMachine<'a, #input_type_name, #output_type_name> for #state_machine_name <'a> {
            type State = #state_enum_name;
            type Configuration = #configuration_type_name;

            fn init(state: Self::State, configuration: &'a Self::Configuration) -> Self {
                Self {
                    state,
                    configuration,
                }
            }

            fn next(&mut self, input: #input_type_name) -> #output_type_name {
                match self.state #impl_body
            }
        }
    };

    // TODO: now that we have impl_body we can also secondarily start generating the lib.rs for compiling state machines
    // TODO: runtime in cycles analysis of compiled branches
    // TODO: strict checking of boundedness of code in state machine branches

    proc_macro::TokenStream::from(expanded)
}
