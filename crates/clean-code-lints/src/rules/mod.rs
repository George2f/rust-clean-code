use std::sync::OnceLock;

use crate::rule::Rule;

pub mod be_precise;
pub mod command_query_separation;
pub mod declaration_proximity;
pub mod descriptive_names;
pub mod encapsulate_boundaries;
pub mod encapsulate_conditionals;
pub mod function_does_one_thing;
pub mod function_names_say_what_they_do;
pub mod law_of_demeter;
pub mod limited_public_api;
pub mod long_names_for_long_scopes;
pub mod max_arguments;
pub mod names_describe_side_effects;
pub mod no_ambiguous_names;
pub mod no_base_class_knowing_derivatives;
pub mod no_clutter;
pub mod no_commented_out_code;
pub mod no_dead_code;
pub mod no_dead_functions;
pub mod no_duplication;
pub mod no_encodings;
pub mod no_feature_envy;
pub mod no_flag_arguments;
pub mod no_inappropriate_comments;
pub mod no_inappropriate_statics;
pub mod no_inherited_constants;
pub mod no_magic_numbers;
pub mod no_multiple_languages;
pub mod no_negative_conditionals;
pub mod no_obscured_intent;
pub mod no_output_arguments;
pub mod no_overridden_safeties;
pub mod no_pass_null;
pub mod no_redundant_comments;
pub mod no_return_null;
pub mod no_selector_arguments;
pub mod no_skipped_tests;
pub mod no_todo_tests;
pub mod one_assertion_per_test;
pub mod one_level_of_abstraction;
pub mod prefer_exceptions_over_error_codes;
pub mod prefer_namespace_imports;
pub mod prefer_polymorphism;
pub mod use_explanatory_variables;

pub fn all() -> &'static [Box<dyn Rule>] {
    static RULES: OnceLock<Vec<Box<dyn Rule>>> = OnceLock::new();
    RULES.get_or_init(build_rules)
}

fn build_rules() -> Vec<Box<dyn Rule>> {
    let mut rules = naming_rules();
    rules.extend(function_rules());
    rules.extend(structure_rules());
    rules.extend(clarity_rules());
    rules.extend(interface_rules());
    rules.extend(discipline_rules());
    rules
}

fn naming_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(descriptive_names::DescriptiveNames),
        Box::new(function_names_say_what_they_do::FunctionNamesSayWhatTheyDo),
        Box::new(long_names_for_long_scopes::LongNamesForLongScopes),
        Box::new(names_describe_side_effects::NamesDescribeSideEffects),
        Box::new(no_ambiguous_names::NoAmbiguousNames),
        Box::new(no_encodings::NoEncodings),
    ]
}

fn function_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(be_precise::BePrecise),
        Box::new(command_query_separation::CommandQuerySeparation),
        Box::new(declaration_proximity::DeclarationProximity),
        Box::new(encapsulate_boundaries::EncapsulateBoundaries),
        Box::new(encapsulate_conditionals::EncapsulateConditionals),
        Box::new(function_does_one_thing::FunctionDoesOneThing),
        Box::new(law_of_demeter::LawOfDemeter),
        Box::new(max_arguments::MaxArguments),
    ]
}

fn structure_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(limited_public_api::LimitedPublicApi),
        Box::new(no_clutter::NoClutter),
        Box::new(no_dead_code::NoDeadCode),
        Box::new(no_dead_functions::NoDeadFunctions),
        Box::new(no_duplication::NoDuplication),
        Box::new(no_feature_envy::NoFeatureEnvy),
        Box::new(no_inappropriate_statics::NoInappropriateStatics),
        Box::new(no_magic_numbers::NoMagicNumbers),
    ]
}

fn clarity_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(no_commented_out_code::NoCommentedOutCode),
        Box::new(no_inappropriate_comments::NoInappropriateComments),
        Box::new(no_multiple_languages::NoMultipleLanguages),
        Box::new(no_redundant_comments::NoRedundantComments),
        Box::new(no_obscured_intent::NoObscuredIntent),
        Box::new(use_explanatory_variables::UseExplanatoryVariables),
    ]
}

fn interface_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(no_base_class_knowing_derivatives::NoBaseClassKnowingDerivatives),
        Box::new(no_flag_arguments::NoFlagArguments),
        Box::new(no_inherited_constants::NoInheritedConstants),
        Box::new(no_output_arguments::NoOutputArguments),
        Box::new(no_overridden_safeties::NoOverriddenSafeties),
        Box::new(no_pass_null::NoPassNull),
        Box::new(no_return_null::NoReturnNull),
        Box::new(no_selector_arguments::NoSelectorArguments),
    ]
}

fn discipline_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(no_negative_conditionals::NoNegativeConditionals),
        Box::new(no_skipped_tests::NoSkippedTests),
        Box::new(no_todo_tests::NoTodoTests),
        Box::new(one_assertion_per_test::OneAssertionPerTest),
        Box::new(one_level_of_abstraction::OneLevelOfAbstraction),
        Box::new(prefer_exceptions_over_error_codes::PreferExceptionsOverErrorCodes),
        Box::new(prefer_namespace_imports::PreferNamespaceImports),
        Box::new(prefer_polymorphism::PreferPolymorphism),
    ]
}

pub fn by_name(name: &str) -> Option<&'static dyn Rule> {
    all()
        .iter()
        .map(Box::as_ref)
        .find(|rule| rule.meta().name == name)
}
