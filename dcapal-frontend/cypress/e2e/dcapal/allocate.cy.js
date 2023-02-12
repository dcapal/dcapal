/// <reference types="cypress" />

describe("allocate flow", () => {
  it("should render properly", () => {
    cy.visit("/allocate");
    
    cy.contains('Choose your portfolio currency')
    
    cy.get('[data-testid="ccy-group"]').should('exist')
    cy.get('[data-testid="next-btn"]')
      .should('exist')
      .should('be.disabled')
    
    
    cy.get('[data-testid="ccy-group"] > :nth-child(1)').click()
    cy.get('[data-testid="next-btn"]')
      .should('exist')
      .should('not.be.disabled')
      .click()

  });
});