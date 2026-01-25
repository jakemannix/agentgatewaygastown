"use strict";
/**
 * Pipeline pattern builder
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.PipelineBuilder = exports.StepBuilder = void 0;
exports.pipeline = pipeline;
exports.step = step;
exports.fromInput = fromInput;
exports.fromStep = fromStep;
exports.constant = constant;
exports.construct = construct;
const path_builder_js_1 = require("../path-builder.js");
/**
 * Builder for pipeline steps
 */
class StepBuilder {
    step = {};
    constructor(id) {
        this.step.id = id;
    }
    /**
     * Set step to call a tool
     */
    tool(name) {
        this.step.operation = { tool: { name } };
        return this;
    }
    /**
     * Set step to execute a pattern
     */
    pattern(spec) {
        this.step.operation = { pattern: spec };
        return this;
    }
    /**
     * Set input from composition input
     */
    fromInput(pathExpr = '$') {
        this.step.input = { input: { path: (0, path_builder_js_1.path)(pathExpr) } };
        return this;
    }
    /**
     * Set input from a previous step
     */
    fromStep(stepId, pathExpr = '$') {
        this.step.input = { step: { stepId, path: (0, path_builder_js_1.path)(pathExpr) } };
        return this;
    }
    /**
     * Set input to a constant value
     */
    constant(value) {
        this.step.input = { constant: value };
        return this;
    }
    /**
     * Construct input from multiple field bindings
     * Allows building a new object from fields extracted from prior steps or input
     */
    construct(fields) {
        this.step.input = { construct: { fields } };
        return this;
    }
    /**
     * Build the step
     */
    build() {
        if (!this.step.id) {
            throw new Error('Step ID is required');
        }
        if (!this.step.operation) {
            throw new Error('Step operation (tool or pattern) is required');
        }
        return this.step;
    }
}
exports.StepBuilder = StepBuilder;
/**
 * Builder for pipeline patterns
 */
class PipelineBuilder {
    steps = [];
    /**
     * Add a step that calls a tool
     */
    step(id, toolName) {
        this.steps.push({
            id,
            operation: { tool: { name: toolName } },
        });
        return this;
    }
    /**
     * Add a step with full configuration
     */
    addStep(step) {
        this.steps.push(step);
        return this;
    }
    /**
     * Add a step using the step builder
     */
    add(id) {
        const builder = new StepBuilder(id);
        builder.then = () => {
            this.steps.push(builder.build());
            return this;
        };
        return builder;
    }
    /**
     * Build the pipeline pattern spec
     */
    build() {
        return { pipeline: { steps: this.steps } };
    }
    /**
     * Get the raw pipeline spec
     */
    spec() {
        return { steps: this.steps };
    }
}
exports.PipelineBuilder = PipelineBuilder;
/**
 * Create a pipeline pattern
 *
 * @example
 * const searchPipeline = pipeline()
 *   .step('search', 'web_search')
 *   .step('summarize', 'summarize_text')
 *   .build();
 *
 * // With step builder
 * const pipeline2 = pipeline()
 *   .add('search').tool('web_search').fromInput('$.query').then()
 *   .add('process').tool('process').fromStep('search', '$.results').then()
 *   .build();
 */
function pipeline() {
    return new PipelineBuilder();
}
/**
 * Create a step builder
 */
function step(id) {
    return new StepBuilder(id);
}
// =============================================================================
// Binding Helper Functions
// =============================================================================
/**
 * Create an input binding (reference to composition input)
 */
function fromInput(pathExpr = '$') {
    return { input: { path: (0, path_builder_js_1.path)(pathExpr) } };
}
/**
 * Create a step binding (reference to prior step output)
 */
function fromStep(stepId, pathExpr = '$') {
    return { step: { stepId, path: (0, path_builder_js_1.path)(pathExpr) } };
}
/**
 * Create a constant binding
 */
function constant(value) {
    return { constant: value };
}
/**
 * Create a construct binding (build object from multiple field bindings)
 *
 * @example
 * construct({
 *   product_id: fromStep('alerts', '$.alerts[0].product_id'),
 *   quantity: fromStep('alerts', '$.alerts[0].deficit'),
 * })
 */
function construct(fields) {
    return { construct: { fields } };
}
//# sourceMappingURL=pipeline.js.map