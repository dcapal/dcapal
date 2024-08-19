export const Button = {
  baseStyle: {
    fontWeight: "normal",
  },
  sizes: {
    lg: {
      py: "8px",
      px: "12px",
    },
  },
  variants: {
    'outline': {
      color: 'black',
      _hover: {
        color: 'white',
        backgroundColor: 'neutral.600'
      },
      _active: {
        color: 'white',
        backgroundColor: 'neutral.800'
      }
    }
  }
};
