import { expect } from "chai";
import { LiteSVM } from "litesvm";

describe("LiteSVM", () => {
    it("should create a new LiteSVM instance", () => {
        const svm = new LiteSVM();
        expect(svm).to.be.an.instanceof(LiteSVM);
    });
});