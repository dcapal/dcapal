/// <reference types="cypress" />

describe("allocate flow", () => {

  it.only('should fillup a basic portfolio',() => {
    cy.visit("/allocate");
      
    cy.contains('Choose your portfolio currency')
    
    cy.get('[data-testid="ccy-group"]').should('exist')
    cy.get('[data-testid="next-btn"]')
      .should('exist')
      .should('be.disabled')
    
      
    cy.get('[data-testid="ccy-group"] > :nth-child(1)').should('have.attr','aria-checked','false')
    cy.get('[data-testid="ccy-group"] > :nth-child(1)').click()
    cy.get('[data-testid="ccy-group"] > :nth-child(1)').should('have.attr','aria-checked','true')

    cy.get('[data-testid="next-btn"]')
      .should('exist')
      .should('not.be.disabled')
      .click()
  })

  describe('edge case', () => {
    it("should keep currency selected through navigation", () => {
      cy.visit("/allocate");
      
      cy.contains('Choose your portfolio currency')
      
      cy.get('[data-testid="ccy-group"]').should('exist')
      cy.get('[data-testid="next-btn"]')
        .should('exist')
        .should('be.disabled')
      
        
      cy.get('[data-testid="ccy-group"] > :nth-child(1)').should('have.attr','aria-checked','false')
      cy.get('[data-testid="ccy-group"] > :nth-child(1)').click()
      cy.get('[data-testid="ccy-group"] > :nth-child(1)').should('have.attr','aria-checked','true')
  
      cy.get('[data-testid="next-btn"]')
        .should('exist')
        .should('not.be.disabled')
        .click()
  
      cy.get('[data-testid="back-btn"]').click()
  
      cy.get('[data-testid="ccy-group"] > :nth-child(1)').should('have.attr','aria-checked','true')
      cy.get('[data-testid="next-btn"]')
        .should('exist')
        .should('not.be.disabled')
    });
  })
});